# MiniMax Provider Fix Plan - ULTRA-DETAILED

## Status: PLANNING

**Date:** 2026-05-28  
**Target:** MiniMax Provider (China endpoint `api.minimaxi.com`)  
**Context:** OpenAI-compatible API structure with China-specific routing via `sk-cp-` key prefix detection

## Validation Update: 2026-05-28

The original plan assumed that every `sk-cp-*` key should route to the China endpoint.
That assumption is not safe: current MiniMax OpenAI-compatible/token-plan docs advertise
`https://api.minimax.io/v1` as the default base URL, and real `sk-cp-*` token-plan keys can
fail with `401` when forced to `https://api.minimaxi.com/v1`.

Implementation direction is therefore changed:

- Do not infer China routing from the key prefix.
- Keep MiniMax on the profile default `https://api.minimax.io/v1`.
- Store new MiniMax credentials under `MINIMAX_API_KEY`; keep reading legacy
  `OPENAI_API_KEY` from `minimax.env` for backward compatibility.
- Do not add fallback from China to international unless there is an explicit user-visible
  China profile or config switch.
- Do not add a local request-count rate limiter yet; provider-side rate-limit headers/errors
  are the safer source of truth.

---

## 1. API Analysis: MiniMax China vs International

### 1.1 Endpoint Comparison

| Region | API Base | API Key Prefix | Rate Limit |
|--------|----------|----------------|------------|
| International | `https://api.minimax.io/v1` | Standard | Same |
| China | `https://api.minimaxi.com/v1` | `sk-cp-*` | 4500 req/5h |

### 1.2 Current Implementation (src/provider_catalog.rs)

```rust
// Lines 97-120: Key-based endpoint override
const MINIMAX_CHINA_API_BASE: &str = "https://api.minimaxi.com/v1";
const MINIMAX_CHINA_SETUP_URL: &str = "https://platform.minimaxi.com/docs/llms.txt";

fn apply_profile_key_based_endpoint_overrides(...) {
    if profile.id != MINIMAX_PROFILE.id { return; }
    
    let key = api_key_hint
        .map(str::trim)
        .filter(|key| !key.is_empty())
        .or_else(|| load_env_value_from_env_or_config(...));
    
    if key.as_deref().map(|key| key.trim_start().starts_with("sk-cp-")).unwrap_or(false) {
        resolved.api_base = MINIMAX_CHINA_API_BASE.to_string();
        resolved.setup_url = MINIMAX_CHINA_SETUP_URL.to_string();
    }
}
```

### 1.3 Test Coverage (src/provider_catalog_tests.rs:111-130)

```rust
#[test]
fn minimax_token_plan_keys_resolve_to_china_endpoint_without_changing_international_default() {
    let international = resolve_openai_compatible_profile(MINIMAX_PROFILE);
    assert_eq!(international.api_base, "https://api.minimax.io/v1");
    
    let china = resolve_openai_compatible_profile_with_api_key_hint(
        MINIMAX_PROFILE,
        Some("sk-cp-test-token"),
    );
    assert_eq!(china.api_base, MINIMAX_CHINA_API_BASE);
}
```

### 1.4 Known Issues

1. **China endpoint may have connectivity issues** - DNS resolution, TLS handshake, or network routing problems
2. **No dedicated testing** - No live tests for `api.minimaxi.com` endpoint
3. **No rate limit awareness** - 4500 requests/5h window not monitored
4. **No fallback strategy** - If China endpoint fails, no fallback to international

---

## 2. Authentication Flow

### 2.1 Current Flow

```
1. User configures MiniMax (jcode login --provider minimax)
   ↓
2. API key stored in ~/.config/jcode/minimax.env as MINIMAX_API_KEY
   ↓
3. resolve_openai_compatible_profile() called
   ↓
4. apply_profile_key_based_endpoint_overrides() checks key prefix
   ↓
5. If key starts with "sk-cp-": switch to api.minimaxi.com
   ↓
6. Provider uses OpenAI-compatible path via openrouter provider
```

### 2.2 Key Files Involved

| File | Role |
|------|------|
| `src/provider_catalog.rs` | Endpoint resolution, `sk-cp-` detection |
| `crates/jcode-provider-metadata/src/catalog.rs` | Profile definitions (MINIMAX_PROFILE) |
| `src/provider/openai.rs` | OpenAI-compatible transport layer |
| `src/provider/openai_stream_runtime.rs` | HTTP transport with retry |
| `src/provider/openai_provider_impl.rs` | Provider implementation |
| `src/auth/external.rs` | API key loading from env files |

### 2.3 Auth Flow for China Keys

```rust
// From src/auth/external.rs line 380
"MINIMAX_API_KEY" => &["minimax"],
```

The key is loaded via `load_api_key_from_env_or_config()` which checks:
1. Environment variable `MINIMAX_API_KEY` (or `OPENAI_API_KEY`)
2. Config file `~/.config/jcode/minimax.env`
3. External auth sources

---

## 3. Rate Limiting: 4500 requests / 5 hours

### 3.1 Current Implementation

**Problem:** No rate limit tracking for MiniMax.

**Rate calculation:**
- 4500 requests / 5 hours = 900 requests/hour
- Typical session (2 hrs active) = 1800 requests
- ~2.5 full sessions per 5h window

### 3.2 Proposed Rate Limit Implementation

**New file:** `src/provider/minimax_rate_limiter.rs`

```rust
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub struct MiniMaxRateLimiter {
    /// Requests in current 5-hour window
    requests_in_window: AtomicU64,
    /// Window start time
    window_start: std::sync::Mutex<Instant>,
    /// Max requests per 5 hours
    max_requests: u64,
}

impl MiniMaxRateLimiter {
    pub fn new(max_requests: u64) -> Self {
        Self {
            requests_in_window: AtomicU64::new(0),
            window_start: std::sync::Mutex::new(Instant::now()),
            max_requests,
        }
    }
    
    /// Check if we can make a request, increment counter if yes
    pub fn can_request(&self) -> bool {
        self.check_and_update_window();
        self.requests_in_window.load(Ordering::Relaxed) < self.max_requests
    }
    
    /// Get remaining requests in current window
    pub fn remaining(&self) -> u64 {
        self.check_and_update_window();
        self.max_requests.saturating_sub(
            self.requests_in_window.load(Ordering::Relaxed)
        )
    }
    
    /// Get seconds until window resets
    pub fn reset_in_secs(&self) -> u64 {
        let start = *self.window_start.lock().unwrap();
        let elapsed = start.elapsed().as_secs();
        18000.saturating_sub(elapsed) // 5 hours = 18000 seconds
    }
    
    fn check_and_update_window(&self) {
        let mut start = self.window_start.lock().unwrap();
        if start.elapsed() > Duration::from_secs(18000) {
            // Reset window
            *start = Instant::now();
            self.requests_in_window.store(0, Ordering::Relaxed);
        }
    }
    
    pub fn record_request(&self) {
        self.requests_in_window.fetch_add(1, Ordering::Relaxed);
    }
}
```

### 3.3 Integration with OpenAI Transport

**In `src/provider/openai_stream_runtime.rs`:**

Add MiniMax-specific rate limit handling:

```rust
// After parsing error response
if let Some(retry_after) = response.get("retry-after") {
    // MiniMax returns retry-after header
    if let Some(s) = retry_after.as_u64() {
        return Err(anyhow!("Rate limit exceeded. Retry after {} seconds", s));
    }
}
```

### 3.4 Rate Limit Logging

Add to `src/logging.rs` or existing logging:

```rust
if provider_id == "minimax" && remaining < 500 {
    crate::logging::warn(&format!(
        "MiniMax rate limit warning: {} requests remaining in current 5h window. Resets in {}s",
        remaining,
        reset_in_secs
    ));
}
```

---

## 4. Fallback Strategy

### 4.1 Current State

No fallback from China to International endpoint.

### 4.2 Proposed Fallback Implementation

**Add to `src/provider_catalog.rs`:**

```rust
pub const MINIMAX_INTERNATIONAL_FALLBACK: &str = "https://api.minimax.io/v1";

/// Attempt to fall back from China to International endpoint
pub fn minimax_china_fallback_available() -> bool {
    let key = load_api_key_from_env_or_config(
        MINIMAX_PROFILE.api_key_env,
        MINIMAX_PROFILE.env_file
    );
    key.map(|k| k.trim_start().starts_with("sk-cp-")).unwrap_or(false)
}
```

### 4.3 Fallback Logic in Transport Layer

**In `src/provider/openai_stream_runtime.rs`:**

```rust
async fn make_request_with_minimax_fallback(&self, request: Request) -> Result<Response> {
    let is_china_key = minimax_china_fallback_available();
    
    if is_china_key {
        // Try China endpoint first
        match self.make_request(request.clone()).await {
            Ok(resp) => return Ok(resp),
            Err(e) if is_connection_error(&e) => {
                crate::logging::warn("MiniMax China endpoint failed, trying International...");
                // Modify request to use international endpoint
                let mut fallback_req = request;
                fallback_req.url = fallback_req.url.replace(
                    "api.minimaxi.com",
                    "api.minimax.io"
                );
                return self.make_request(fallback_req).await;
            }
            Err(e) => return Err(e),
        }
    }
    
    self.make_request(request).await
}
```

### 4.4 Connection Error Detection

```rust
fn is_connection_error(e: &anyhow::Error) -> bool {
    let msg = e.to_string().to_lowercase();
    msg.contains("connection")
        || msg.contains("timeout")
        || msg.contains("dns")
        || msg.contains("network")
        || msg.contains("ssl")
        || msg.contains("tls")
        || msg.contains("certificate")
        || msg.contains("name or service not known")
        || msg.contains("connection refused")
}
```

---

## 5. Testing Plan

### 5.1 Unit Tests

**Existing tests in `src/provider_catalog_tests.rs`:**
- `minimax_token_plan_keys_resolve_to_china_endpoint_without_changing_international_default`

**New tests to add:**

```rust
#[test]
fn minimax_china_key_detection_handles_whitespace() {
    let china = resolve_openai_compatible_profile_with_api_key_hint(
        MINIMAX_PROFILE,
        Some("  sk-cp-key-with-spaces  "),
    );
    assert_eq!(china.api_base, MINIMAX_CHINA_API_BASE);
}

#[test]
fn minimax_international_key_does_not_switch() {
    let intl = resolve_openai_compatible_profile_with_api_key_hint(
        MINIMAX_PROFILE,
        Some("sk-regular-international-key"),
    );
    assert_eq!(intl.api_base, "https://api.minimax.io/v1");
}

#[test]
fn minimax_empty_key_uses_default() {
    let default = resolve_openai_compatible_profile(MINIMAX_PROFILE);
    assert_eq!(default.api_base, "https://api.minimax.io/v1");
}
```

### 5.2 Integration Tests

**New file:** `src/provider_tests/minimax_integration_tests.rs`

```rust
#[tokio::test]
#[ignore] // Requires real API key
async fn minimax_china_endpoint_health_check() {
    // Test that api.minimaxi.com is reachable
    let client = reqwest::Client::new();
    let resp = client
        .get("https://api.minimaxi.com/v1/models")
        .header("Authorization", format!("Bearer {}", get_test_key()))
        .send()
        .await;
    
    assert!(resp.is_ok(), "China endpoint should be reachable");
    assert_eq!(resp.unwrap().status(), 200);
}

#[tokio::test]
#[ignore] // Requires real API key
async fn minimax_international_fallback_works() {
    // If China fails, should be able to fall back to international
    // (Only works if key is valid for both, which it isn't - so just verify behavior)
}
```

### 5.3 Test Configuration

**In `tests/live_tests.rs`:**

```rust
#[test]
#[ignore = "Requires MINIMAX_API_KEY environment variable"]
fn minimax_provider_end_to_end() {
    let api_key = std::env::var("MINIMAX_API_KEY").expect("Set MINIMAX_API_KEY");
    let is_china = api_key.starts_with("sk-cp-");
    
    let expected_endpoint = if is_china {
        "https://api.minimaxi.com/v1"
    } else {
        "https://api.minimax.io/v1"
    };
    
    let resolved = resolve_openai_compatible_profile_with_api_key_hint(
        MINIMAX_PROFILE,
        Some(&api_key)
    );
    
    assert_eq!(resolved.api_base, expected_endpoint);
}
```

### 5.4 Test Execution

```bash
# Run MiniMax-specific tests
cargo test minimax --no-fail-fast

# Run integration tests (requires API key)
MINIMAX_API_KEY=sk-cp-xxx cargo test minimax_live -- --ignored

# Run rate limiter tests
cargo test rate_limiter --no-fail-fast
```

---

## 6. Monitoring

### 6.1 Logging Strategy

**Add structured logging for MiniMax:**

```rust
// In src/logging.rs or existing log calls
if provider_id == "minimax" {
    // Log request with endpoint info
    crate::logging::info(&format!(
        "MiniMax request: endpoint={} model={} key_prefix={}",
        endpoint,
        model,
        if api_key.starts_with("sk-cp-") { "china" } else { "international" }
    ));
    
    // Log rate limit status periodically
    if request_count % 100 == 0 {
        let remaining = rate_limiter.remaining();
        crate::logging::info(&format!(
            "MiniMax usage: {} requests remaining, resets in {}s",
            remaining,
            rate_limiter.reset_in_secs()
        ));
    }
}
```

### 6.2 Latency Tracking

**Current:** `.jcode/.jcode_latency_log.jsonl` already tracks MiniMax.

**Enhancement:** Add endpoint-specific tracking:

```json
{"timestamp":"2026-05-28T12:00:00","provider":"minimax","endpoint":"api.minimaxi.com","jcode_ms":77,"api_ms":150}
```

### 6.3 Error Monitoring

Track connection failures to China endpoint:

```rust
if endpoint.contains("minimaxi.com") {
    crate::metrics::increment("minimax_china_connection_errors");
    
    // Log for debugging
    crate::logging::debug(&format!(
        "MiniMax China connection error: {} (attempt {})",
        error,
        attempt
    ));
}
```

### 6.4 Rate Limit Exhaustion Warning

```rust
if provider_id == "minimax" && remaining < 100 {
    crate::logging::warn(&format!(
        "MiniMax rate limit critical: only {} requests remaining",
        remaining
    ));
}
```

---

## 7. Code Changes

### 7.1 Files to Modify

| File | Change Description | Priority |
|------|-------------------|----------|
| `src/provider_catalog.rs` | Add rate limiter integration | HIGH |
| `src/provider/openai_stream_runtime.rs` | Add China fallback logic, rate limit handling | HIGH |
| `src/provider_catalog_tests.rs` | Add more MiniMax-specific tests | MEDIUM |
| `crates/jcode-provider-metadata/src/catalog.rs` | No changes needed (already correct) | N/A |
| `src/logging.rs` | Add MiniMax-specific logging | LOW |

### 7.2 New Files

| File | Purpose | Priority |
|------|---------|----------|
| `src/provider/minimax_rate_limiter.rs` | Rate limit tracking for 4500 req/5h | HIGH |
| `src/provider_tests/minimax_integration_tests.rs` | Integration tests for China endpoint | MEDIUM |

### 7.3 Detailed Changes

#### 7.3.1 src/provider_catalog.rs

**Add imports:**
```rust
#[cfg(feature = "minimax-rate-limit")]
mod minimax_rate_limiter;
```

**Add rate limiter initialization:**
```rust
#[cfg(feature = "minimax-rate-limit")]
use std::sync::LazyLock;
#[cfg(feature = "minimax-rate-limit")]
static MINIMAX_RATE_LIMITER: LazyLock<Arc<MinimaxRateLimiter>> = 
    LazyLock::new(|| Arc::new(MinimaxRateLimiter::new(4500)));
```

#### 7.3.2 src/provider/openai_stream_runtime.rs

**Add MiniMax fallback handling:**

After line ~150 where retry logic exists:

```rust
// MiniMax China endpoint fallback
if api_base.contains("minimaxi.com") && is_retriable && attempt < MAX_RETRIES {
    // Check if error is connection-related
    if is_connection_error(&error) {
        crate::logging::warn(&format!(
            "MiniMax China endpoint failed: {}. Attempting international fallback...",
            error
        ));
        
        // Retry with international endpoint
        let fallback_url = api_base.replace("minimaxi.com", "minimax.io");
        let retry_req = build_request_with_url(request, &fallback_url);
        return execute_with_backoff(retry_req, attempt + 1).await;
    }
}
```

#### 7.3.3 New: src/provider/minimax_rate_limiter.rs

```rust
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

const WINDOW_DURATION_SECS: u64 = 18000; // 5 hours

pub struct MiniMaxRateLimiter {
    requests_in_window: AtomicU64,
    window_start: std::sync::Mutex<Instant>,
    max_requests: u64,
}

impl MiniMaxRateLimiter {
    pub fn new(max_requests: u64) -> Self {
        Self {
            requests_in_window: AtomicU64::new(0),
            window_start: std::sync::Mutex::new(Instant::now()),
            max_requests,
        }
    }
    
    pub fn can_request(&self) -> bool {
        self.check_and_update_window();
        self.requests_in_window.load(Ordering::Relaxed) < self.max_requests
    }
    
    pub fn remaining(&self) -> u64 {
        self.check_and_update_window();
        self.max_requests.saturating_sub(
            self.requests_in_window.load(Ordering::Relaxed)
        )
    }
    
    pub fn reset_in_secs(&self) -> u64 {
        let start = *self.window_start.lock().unwrap();
        let elapsed = start.elapsed().as_secs();
        WINDOW_DURATION_SECS.saturating_sub(elapsed)
    }
    
    fn check_and_update_window(&self) {
        let mut start = self.window_start.lock().unwrap();
        if start.elapsed() > Duration::from_secs(WINDOW_DURATION_SECS) {
            *start = Instant::now();
            self.requests_in_window.store(0, Ordering::Relaxed);
        }
    }
    
    pub fn record_request(&self) {
        self.requests_in_window.fetch_add(1, Ordering::Relaxed);
    }
}
```

#### 7.3.4 src/provider_catalog_tests.rs - Add Tests

```rust
#[test]
fn minimax_china_fallback_is_possible() {
    // Verify fallback endpoint is accessible (mocked test)
    assert_eq!(
        MINIMAX_CHINA_API_BASE,
        "https://api.minimaxi.com/v1"
    );
    assert_eq!(
        "https://api.minimax.io/v1",
        MINIMAX_PROFILE.api_base
    );
}

#[test]
fn minimax_key_prefix_detection() {
    let test_cases = vec![
        ("sk-cp-abcd1234", true),
        ("sk-cp-", true),
        ("sk-regular-key", false),
        ("", false),
        ("  sk-cp-key  ", true),
    ];
    
    for (key, expected_china) in test_cases {
        let resolved = resolve_openai_compatible_profile_with_api_key_hint(
            MINIMAX_PROFILE,
            Some(key),
        );
        let is_china = resolved.api_base == MINIMAX_CHINA_API_BASE;
        assert_eq!(is_china, expected_china, "Key: {}", key);
    }
}
```

---

## 8. Request Coalescing

### 8.1 Purpose

When rate limit is near exhaustion, coalesce similar requests to avoid waste.

### 8.2 Implementation

```rust
use std::collections::HashMap;
use tokio::sync::Mutex;

pub struct RequestCoalescer {
    pending: HashMap<String, tokio::sync::oneshot::Sender<Response>>,
    pending_lock: Mutex<()>,
}

impl RequestCoalescer {
    /// Coalesce duplicate requests within a time window
    pub async fn coalesce<F, Fut>(&self, key: String, factory: F) -> Response
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Response>,
    {
        // Check if similar request is pending
        {
            let pending = self.pending.lock().await;
            if let Some(sender) = pending.get(&key) {
                // Wait for existing request
                let receiver = sender.clone();
                drop(pending);
                return receiver.await.unwrap_or_else(|_| factory().await);
            }
        }
        
        // Execute request
        let response = factory().await;
        
        // Notify waiting requests
        {
            let mut pending = self.pending.lock().await;
            if let Some(sender) = pending.remove(&key) {
                let _ = sender.send(response.clone());
            }
        }
        
        response
    }
}
```

### 8.3 Integration

Add to `src/provider/openai_stream_runtime.rs`:

```rust
static REQUEST_COALESCER: LazyLock<Arc<RequestCoalescer>> = 
    LazyLock::new(|| Arc::new(RequestCoalescer::new()));
```

---

## 9. Error Handling Summary

| Error Type | Handling | Retry |
|------------|----------|-------|
| 429 Rate Limit | Wait for retry-after, use rate limiter | Yes (after wait) |
| Connection Timeout | Try international fallback | Yes |
| DNS Failure | Try international fallback | Yes |
| 401 Auth Error | Don't retry, report to user | No |
| 500 Server Error | Exponential backoff | Yes (3 attempts) |
| Network Unreachable | Try international fallback | Yes |

---

## 10. Implementation Order

1. **Phase 1: Core Fixes**
   - Add rate limiter module
   - Add China fallback logic
   - Test endpoint resolution

2. **Phase 2: Testing**
   - Add unit tests
   - Add integration tests
   - Verify fallback behavior

3. **Phase 3: Monitoring**
   - Add structured logging
   - Add rate limit warnings
   - Add latency tracking

4. **Phase 4: Polish**
   - Request coalescing (if needed)
   - Performance optimization
   - Documentation

---

## 11. Files Summary

### Core Files
- `src/provider_catalog.rs` - Endpoint resolution (MODIFY)
- `crates/jcode-provider-metadata/src/catalog.rs` - Profile definitions (OK)

### Transport Layer
- `src/provider/openai_stream_runtime.rs` - HTTP transport with fallback (MODIFY)
- `src/provider/openai_provider_impl.rs` - Provider impl (MODIFY for rate limit)

### New Files
- `src/provider/minimax_rate_limiter.rs` - Rate limit tracking (CREATE)
- `src/provider_tests/minimax_integration_tests.rs` - Integration tests (CREATE)

### Tests
- `src/provider_catalog_tests.rs` - Unit tests (MODIFY)

---

## 12. Validation Criteria

1. **Endpoint Detection:**
   - `sk-cp-*` keys resolve to `api.minimaxi.com`
   - Regular keys resolve to `api.minimax.io`

2. **Rate Limiting:**
   - Track request count per 5-hour window
   - Log warnings when < 500 requests remaining
   - Stop requests when limit reached

3. **Fallback:**
   - Connection errors to China trigger international fallback
   - Fallback only for connection-level errors
   - Auth errors do NOT trigger fallback

4. **Testing:**
   - All existing tests pass
   - New MiniMax-specific tests pass
   - Manual verification with real China key

---

*Plan generated: 2026-05-28*
*Author: Senior Backend Engineer*
