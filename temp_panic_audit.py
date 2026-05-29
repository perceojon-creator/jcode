#!/usr/bin/env python3
"""Exact copy of panic budget scanner logic + detailed match reporting for audit."""
import re
import json
from pathlib import Path
from typing import Any

REPO_ROOT = Path('.').resolve()
SCAN_ROOTS = (REPO_ROOT / 'src', REPO_ROOT / 'crates')
PATTERN = re.compile(r'\.unwrap\(|\.expect\(|\b(?:panic!|todo!|unimplemented!)')
CFG_TEST_RE = re.compile(r'^\s*#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]')
ITEM_START_RE = re.compile(r'^\s*(?:pub(?:\([^)]*\))?\s+)?(?:mod|fn)\b')

def is_test_rust_file(path: Path) -> bool:
    rel = path.relative_to(REPO_ROOT).as_posix()
    if path.suffix != '.rs':
        return False
    parts = rel.split('/')
    if parts[0] == 'tests' or any(
        part == 'tests' or part.endswith('_tests') or part.endswith('_test') or part.startswith('tests_')
        for part in parts
    ):
        return True
    name = path.name
    return (
        name == 'tests.rs'
        or name.endswith('_tests.rs')
        or name.endswith('_test.rs')
        or name.startswith('tests_')
    )

def production_rust_files():
    files = []
    for root in SCAN_ROOTS:
        if not root.exists():
            continue
        for path in sorted(root.rglob('*.rs')):
            if path.suffix == '.rs' and not is_test_rust_file(path):
                files.append(path)
    return files

def brace_delta(line: str) -> int:
    return line.count('{') - line.count('}')

def production_lines(path: Path):
    lines = path.read_text(encoding='utf-8', errors='ignore').splitlines()
    output = []
    skip_stack = []
    pending_cfg_test = False
    for line in lines:
        stripped = line.strip()
        current_depth = sum(skip_stack)
        if current_depth == 0:
            if pending_cfg_test and ITEM_START_RE.match(line):
                delta = brace_delta(line)
                if delta > 0:
                    skip_stack.append(delta)
                pending_cfg_test = False
                continue
            if pending_cfg_test and stripped and not stripped.startswith('#'):
                pending_cfg_test = False
            if CFG_TEST_RE.match(line):
                pending_cfg_test = True
                continue
            output.append(line)
        else:
            skip_stack[-1] += brace_delta(line)
            if skip_stack[-1] <= 0:
                skip_stack.pop()
    return output

def find_matches_in_production(path: Path):
    """Return list of (line_num, stripped_line) for matches in production_lines only."""
    prod_lines = production_lines(path)
    matches = []
    # We need original line numbers. Re-read and map.
    all_lines = path.read_text(encoding='utf-8', errors='ignore').splitlines()
    prod_set = set(prod_lines)  # rough, but since we need numbers, better reprocess
    # Better: simulate and track original indices
    output_indices = []
    skip_stack = []
    pending_cfg_test = False
    for idx, line in enumerate(all_lines):
        stripped = line.strip()
        current_depth = sum(skip_stack)
        if current_depth == 0:
            if pending_cfg_test and ITEM_START_RE.match(line):
                delta = brace_delta(line)
                if delta > 0:
                    skip_stack.append(delta)
                pending_cfg_test = False
                continue
            if pending_cfg_test and stripped and not stripped.startswith('#'):
                pending_cfg_test = False
            if CFG_TEST_RE.match(line):
                pending_cfg_test = True
                continue
            output_indices.append(idx)
        else:
            skip_stack[-1] += brace_delta(line)
            if skip_stack[-1] <= 0:
                skip_stack.pop()
    for i in output_indices:
        line = all_lines[i]
        if PATTERN.search(line):
            matches.append((i+1, line.strip()[:160]))
    return matches

files = production_rust_files()
per_file = {}
total = 0
detailed = []
for f in files:
    matches = find_matches_in_production(f)
    c = len(matches)
    if c > 0:
        rel = f.relative_to(REPO_ROOT).as_posix()
        per_file[rel] = c
        total += c
        detailed.append((rel, matches))

print('=== PANIC BUDGET COMPLIANT PRODUCTION COUNT (using official filter) ===')
print('TOTAL PRODUCTION PANIC-PRONE (filtered):', total)
print('FILES WITH HITS:', len(per_file))
print()

print('=== CURRENT VS BASELINE ===')
baseline_path = REPO_ROOT / 'scripts' / 'panic_budget.json'
baseline = json.loads(baseline_path.read_text()) if baseline_path.exists() else {'total':0, 'tracked_files':{}}
print('Baseline total:', baseline.get('total'))
print('Current filtered total:', total)
print()

print('=== ALL FILES WITH PRODUCTION COUNTS (sorted desc) ===')
for rel, c in sorted(per_file.items(), key=lambda x: -x[1]):
    old = baseline.get('tracked_files', {}).get(rel, 'NEW')
    status = 'NEW' if old == 'NEW' or old is None else f'was {old}'
    print(f'{rel}: {c} (baseline {status})')

print()
print('=== DETAILED MATCHES PER FILE (production lines only) ===')
for rel, matches in sorted(detailed, key=lambda x: -len(x[1])):
    print(f'\n{rel} ({len(matches)}):')
    for ln, txt in matches:
        print(f'  {ln}: {txt}')
