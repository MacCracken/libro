#!/usr/bin/env bash
# Run criterion benchmarks and append results to benchmark-results/history.md
#
# Usage: ./scripts/run-benchmarks.sh [-- <extra cargo bench args>]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
RESULTS_DIR="$PROJECT_DIR/benchmark-results"

mkdir -p "$RESULTS_DIR"

TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
RUST_VERSION=$(rustc --version)
COMMIT=$(git -C "$PROJECT_DIR" rev-parse --short HEAD 2>/dev/null || echo "unknown")

echo "=== Libro Benchmark Run ==="
echo "Time:   $TIMESTAMP"
echo "Rust:   $RUST_VERSION"
echo "Commit: $COMMIT"
echo ""

# Run benchmarks, capture output
BENCH_OUTPUT=$(cargo bench --bench chain "$@" 2>&1) || true

echo "$BENCH_OUTPUT"

# Parse criterion output into structured results
# Criterion lines look like: "bench_name       time:   [1.234 µs 1.345 µs 1.456 µs]"
LATEST_JSON="$RESULTS_DIR/latest.json"
LATEST_MD="$RESULTS_DIR/latest.md"
HISTORY_MD="$RESULTS_DIR/history.md"
HISTORY_JSON="$RESULTS_DIR/history.json"

# Build JSON results
python3 -c "
import json, re, sys

output = '''$BENCH_OUTPUT'''

results = []
for line in output.split('\n'):
    m = re.match(r'\s*(\S+)\s+time:\s+\[([0-9.]+)\s+(\w+)\s+([0-9.]+)\s+(\w+)\s+([0-9.]+)\s+(\w+)\]', line)
    if m:
        name = m.group(1)
        low = float(m.group(2))
        unit_low = m.group(3)
        mid = float(m.group(4))
        unit_mid = m.group(5)
        high = float(m.group(6))
        unit_high = m.group(7)

        def to_us(val, unit):
            if unit in ('ns', 'ns/iter'):
                return val / 1000.0
            if unit in ('µs', 'us'):
                return val
            if unit == 'ms':
                return val * 1000.0
            if unit == 's':
                return val * 1_000_000.0
            return val

        results.append({
            'name': name,
            'low_us': round(to_us(low, unit_low), 3),
            'mid_us': round(to_us(mid, unit_mid), 3),
            'high_us': round(to_us(high, unit_high), 3),
            'unit': 'µs',
        })

report = {
    'timestamp': '$TIMESTAMP',
    'rust_version': '$RUST_VERSION',
    'commit': '$COMMIT',
    'benchmarks': results,
}

# Write latest.json
with open('$LATEST_JSON', 'w') as f:
    json.dump(report, f, indent=2)
    f.write('\n')

# Append to history.json
history = []
try:
    with open('$HISTORY_JSON') as f:
        history = json.load(f)
except (FileNotFoundError, json.JSONDecodeError):
    pass
history.append(report)
with open('$HISTORY_JSON', 'w') as f:
    json.dump(history, f, indent=2)
    f.write('\n')

# Build markdown
lines = []
lines.append(f'## {report[\"timestamp\"]} — {report[\"commit\"]}')
lines.append(f'Rust: {report[\"rust_version\"]}')
lines.append('')
lines.append('| Benchmark | Low (µs) | Mid (µs) | High (µs) |')
lines.append('|-----------|----------|----------|-----------|')
for b in results:
    lines.append(f'| {b[\"name\"]} | {b[\"low_us\"]:.1f} | {b[\"mid_us\"]:.1f} | {b[\"high_us\"]:.1f} |')
lines.append('')
md_section = '\n'.join(lines)

# Write latest.md
with open('$LATEST_MD', 'w') as f:
    f.write('# Latest Benchmark Results\n\n')
    f.write(md_section)

# Append to history.md
header_needed = True
try:
    with open('$HISTORY_MD') as f:
        existing = f.read()
        header_needed = not existing.strip()
except FileNotFoundError:
    existing = ''
with open('$HISTORY_MD', 'a') as f:
    if header_needed:
        f.write('# Benchmark History\n\n')
    f.write(md_section)

print('\nResults saved to $RESULTS_DIR/')
" 2>&1

echo ""
echo "=== Done ==="
