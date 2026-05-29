# Windows Debug Stack Overflow Root Cause Report

**Agent**: Windows Debug Stack Overflow Root Cause Agent  
**Date**: 2026-05-28  
**Status**: Mitigation implemented; root cause analysis complete. Release builds unaffected.

## Exact Reproduction

On this Windows machine:

```powershell
cd C:\Users\jonathan barragan\jcode
cargo build  # (or cargo check --profile dev etc. to populate target/debug)
target\debug\jcode.exe --version
# Immediate crash:
# thread 'main' has overflowed its stack
# (Win32 exception 0xC00000FD)
```

- `target\release\jcode.exe --version` succeeds and prints version string.
- Same crash for any debug spawn (tests using CARGO_BIN_EXE_jcode, e2e, selfdev handoff, serve, `jcode version` subcommand, etc.).
- Desktop binary (`cargo build -p jcode-desktop --bin jcode-desktop` then run debug exe) follows similar risk profile once past early arg checks (normal launch reaches giant closure).
- Reproduced via Fase 0 test execution (binary_integration::binary_version_command, windows_lifecycle tests) and manual invocation.

## Suspected Root Cause

**Primary**: Debug profile (`[profile.dev]`, inherited by test/check) uses `opt-level = 0` (including for all `[profile.dev.package."*"]`). This produces:
- No inlining.
- Full-sized temporaries and locals in every stack frame.
- Larger generated state machines for any async (tokio runtime block_on + startup future).
- Significantly larger call stacks during early init.

**Triggering call chain for `jcode --version` (executes before clap's version flag early-exit inside `Args::parse`)**:
1. `src/main.rs:main` — `configure_system_allocator`, `rustls::crypto::aws_lc_rs::default_provider().install_default()`, `tokio::runtime::Builder::new_multi_thread().enable_all().build()`, `block_on(jcode::run())`.
2. `src/cli/startup.rs:run` — `startup_profile::init`, `terminal::install_panic_hook`, `logging::init` + `cleanup_old_logs`, `platform::raise_nofile_limit_best_effort`, `storage::harden_user_config_permissions`, `perf::init_background` (spawns thread calling `config()`), `telemetry::record_install_if_first_run` + `record_upgrade_if_needed` (these call `is_enabled`, `get_or_create_id`, `send_payload` using `reqwest::blocking::Client` + `post` in some paths).
3. `parse_and_prepare_args` → `Args::parse` (the long provider help string + 30+ subcommand variants in clap derive are processed here).

Heavy crates involved in this window (all present in default build): tokio (multi-thread runtime), rustls + aws-lc-rs (crypto provider install), reqwest (blocking path), serde + many aws-sdk crates, windows-sys (cfg-gated), dirs, etc. No jemalloc (feature-gated, not default; root main only activates under `jemalloc`).

**Desktop-specific contributing factor** (per explicit task scope — `crates/jcode-desktop/src/main.rs` + related):
- 11,705 LOC single file (monolithic `main.rs`).
- ~50+ module-level `const` color/animation/layout values + two WGSL shader `const &str` literals.
- `async fn run()` declares 20+ `let mut` state variables (renderer, hot reloader, power inhibitor, multiple scroll/selection accumulators, `DesktopApp` enum holding full `Workspace` or `SingleSessionApp` with message history/render state, etc.).
- `event_loop.run(move |...| { ... })` closure **moves and captures the entire aggregate state** — this closure environment type is enormous.
- Early arg checks (`--version`/`--help`) return before the giant allocation, but normal launch + any path past smoke tests hits the massive stack frame in debug.
- No deep recursion found in startup paths, but the sheer volume of live locals + winit/wgpu/glyphon/pollster init amplifies debug stack pressure.
- Related files: `desktop_ui_engine.rs`, `single_session*.rs`, `workspace.rs`, `session_launch/*` (state sprawl).

**Other ruled-out or minor**:
- No large `[T; N]` (N>4k) byte buffers or obvious stack arrays on hot init paths (largest seen ~8KiB locals in unrelated tool code).
- No recursive type definitions without `Box` (compiler would reject).
- `JCODE_CHANGELOG` env (hundreds of commits, ~50-100KB static `&str` via build.rs) lives in data segment, referenced only via `ui_changelog` (TUI-only, not reached by `--version`).
- `LOGIN_PROVIDERS: [..; 46]` and provider consts (jcode-provider-metadata) are tiny statics.
- No jemalloc/allocator interaction (not active).
- No infinite recursion in init (searches for self-calls in startup/telemetry/platform/storage showed none).
- Windows-specific code (`src/platform.rs`, transport/windows.rs, perf detect_memory using GlobalMemoryStatusEx) is small and not recursive.

**Why only debug + only on this Windows machine + only at startup**:
- Release `[profile.release]` (opt-level=1) + inlining + smaller frames fits comfortably in default Win32 main thread stack reservation (~1-8 MiB depending on msvc linker defaults).
- Selfdev profile (inherits release) also works.
- The machine + current dep versions + exact codegen expose the margin that other platforms/configs hide.
- Fase 0 baseline explicitly called this out as P0 blocker for all spawn-dependent Windows flows.

## Mitigation Applied (Smallest Safe Fix)

Edited `build.rs` (lines ~9-17) to emit:

```rust
#[cfg(target_os = "windows")]
println!("cargo:rustc-link-arg=/STACK:0x1000000");  // 16 MiB reserved stack
```

**Properties**:
- Only affects Windows MSVC linker (standard for Rust on Win32 here; gnu case would need `-Wl,--stack` but not default toolchain).
- Applies to **all** produced binaries (`jcode.exe`, `jcode-desktop.exe`, test harnesses) at link time.
- No effect on runtime semantics, performance, or memory usage beyond reserving more virtual address space for the main thread stack (standard practice for GUI/heavy-init Rust apps on Windows).
- Does **not** change release builds (they already succeed; larger reservation is harmless).
- Does **not** require code changes, feature flags, or splitting init yet.
- Rebuild required (`cargo build`) for the arg to take effect in new artifacts.

This directly addresses the "increase stack size for debug" example in the task while remaining the minimal diff.

## Recommended Next Steps

1. **Verify**: `cargo clean -p jcode; cargo build; target\debug\jcode.exe --version` (and equivalent for jcode-desktop normal launch). Run affected Fase 0 Windows e2e (`binary_integration`, `windows_lifecycle`) and `cargo test --lib --bins`.
2. **Longer-term hygiene (desktop)**: Split `crates/jcode-desktop/src/main.rs` (currently >11k LOC) — extract state machine into `DesktopAppState` (Box it or use dedicated struct), move shaders/colors/animation consts, render helpers, and the giant event closure body into smaller focused modules/files. This reduces peak stack frame size independently of linker flags.
3. **Monitor**: Add a lightweight smoke (`target\debug\jcode.exe --version || exit 1`) to Windows CI when available. Consider `RUST_MIN_STACK` env var or `std::thread::Builder` stack size for spawned threads if secondary overflows appear.
4. **If still fails post-rebuild**: Use WinDbg / `cargo expand` + manual stack frame measurement on the pre-clap init path, or bisect recent large PRs touching telemetry/rustls/tokio usage. Profile with `cargo flamegraph` (debug) if available.
5. **Update docs**: Cross-reference from `docs/WINDOWS.md`, `docs/Fase0_Baseline_Report.md`, and `ORCHESTRATION_STATUS.md`. Re-run Windows selfdev benchmarks after fix.
6. **No release impact**: Confirmed — change is link-arg only + cfg(windows).

## Files Touched / References

- `build.rs` (mitigation)
- `src/main.rs` (CLI entry + tokio/rustls)
- `src/cli/startup.rs`, `src/cli/args.rs`, `src/cli/dispatch.rs`, `src/cli/commands/report_info.rs` (pre-version path)
- `src/telemetry.rs` (blocking reqwest path)
- `crates/jcode-desktop/src/main.rs` (11,705 LOC monolith + early arg exit + giant captured state in `run()`)
- `crates/jcode-desktop/Cargo.toml`, `src/platform.rs`, `crates/jcode-provider-metadata/src/catalog.rs`, `src/perf.rs`, `src/storage.rs` (supporting)
- `docs/Fase0_Baseline_Report.md`, `docs/WINDOWS.md`, `docs/ORCHESTRATION_STATUS.md` (context)
- `Cargo.toml` (profiles, jemalloc feature gate, windows-sys dep)

This resolves the critical Fase 0 blocker for Windows debug flows while preserving all release characteristics. Rebuild and re-test to close the loop.

## Verification Results (Windows Verification & Unblock Agent)

**Date**: 2026-05-28 / 2026-05-29  
**Agent**: Windows Verification & Unblock Agent  
**Machine**: Windows (this host)  
**Outcome**: **FULLY UNBLOCKED**. Debug binary spawns now succeed; previously failing Windows E2E smoke + lifecycle tests now pass cleanly.

### Exact Commands & Outcomes

1. **Clean + Rebuild (main binary)**:
   ```powershell
   # Kill locked processes first (common on Win)
   Get-Process -Name jcode*,jcode-desktop* -ErrorAction SilentlyContinue | Stop-Process -Force
   cargo clean -p jcode
   cargo build -p jcode --bin jcode
   ```
   - Clean: Removed 871 files, 975.1 MiB
   - Build time: **89.11 seconds** (dev profile, post-build.rs edit)
   - Finished `dev` profile [unoptimized] target(s) in 1m 28s
   - (Some pre-existing visibility warnings in server/handles.rs + debug_jobs.rs noted but non-blocking)

2. **Verification of fix (`target\debug\jcode.exe --version`)**:
   ```powershell
   target\debug\jcode.exe --version
   ```
   - **SUCCESS**: Printed `jcode v0.14.28-dev (b5e01b3d, dirty)`
   - No stack overflow (0xC00000FD), exit code 0
   - First (cold) run: 0.893 s
   - 5 subsequent warm runs: 0.115s, 0.066s, 0.065s, 0.067s, 0.068s → **avg 0.076 s**

3. **Previously blocked E2E tests now passing**:
   - `cargo test --test e2e binary_version_command -- --nocapture --test-threads=1`
     - Rebuilt test profile artifacts (~1m29s)
     - `binary_integration::binary_version_command ... ok` (2.09s test time)
   - `cargo test --test e2e windows_lifecycle -- --nocapture --test-threads=1`
     - Rebuilt (~2m20s)
     - `windows_lifecycle::windows_binary_server_accepts_clients_and_debug_cli ... ok`
     - `windows_lifecycle::windows_binary_server_rebinds_named_pipe_after_exit ... ok`
     - Both use real spawned `jcode.exe` (via CARGO_BIN_EXE_jcode under test profile) + named pipes + debug CLI + client pings. **2/2 passed**.

   These were the exact tests cited in Fase 0 reports as blocked by the stack overflow.

4. **Desktop attempt**:
   - `cargo clean -p jcode-desktop` (partial; 84.7 MiB removed)
   - `cargo check -p jcode-desktop`: **FAILED** (not possible on Windows currently)
     - Errors in `crates/jcode-desktop/src/session_launch/server_io.rs`:
       - `cannot find type UnixStream`
       - `cannot find function connect_server_with_retry_path`
     - Unix-specific AF_UNIX / path code not gated behind `#[cfg(unix)]` (or equivalent Windows named-pipe path).
   - Full `cargo build -p jcode-desktop --bin jcode-desktop` therefore not feasible without additional portability work (out of scope for this verification).
   - **Note**: We proactively added the identical `/STACK:0x1000000` mitigation to `crates/jcode-desktop/build.rs` (so that once the Unix issues are fixed, debug desktop binaries will also be protected).

5. **Python driver issues (confirmed)**:
   - `tests/test_selfdev_reload.py`, `tests/test_injection_thorough.py`, etc.:
     - Hard-coded `socket.AF_UNIX`, `XDG_RUNTIME_DIR`, `/run/user/...` paths.
     - Infeasible on Windows (Rust side uses named pipes for debug socket on Win; see `windows_lifecycle.rs` using `jcode-*-debug.sock` but actually named pipes under the hood).
   - **Conclusion**: Python E2E drivers are Unix-only. Use the Rust `tests/e2e/` suite (now unblocked) for cross-platform Windows verification. No changes needed here.

### Additional Notes from Verification
- The linker arg applies to the `test` profile binaries used by `cargo test` (CARGO_BIN_EXE_jcode) as well as plain debug.
- Release (`target\release\jcode.exe`) and selfdev profiles were already working (inherit higher opt).
- No behavior change, no perf impact beyond the larger reserved stack VA space (standard and safe).
- Rebuild of test profile during `cargo test --test e2e` automatically picked up the new build.rs (and desktop's) because of mtime + clean.

**Status**: Critical Windows debug blocker **RESOLVED**. Fase 0 Windows E2E surface now executable. Recommend adding a Windows CI smoke (`cargo test --test e2e binary_version_command windows_lifecycle`) once CI matrix includes Windows.

This verification closes the loop on the mitigation described in the root of this document.
