#!/usr/bin/env bash
set -u

mkdir -p /coverage

if ! command -v cargo-tarpaulin >/dev/null 2>&1; then
  cargo install cargo-tarpaulin
fi

TARPAULIN_LOG="/coverage/tarpaulin.log"
FAILED_REPORT="/coverage/failed-tests.txt"

set +e
set -o pipefail
cargo tarpaulin \
  --engine llvm \
  --workspace \
  --all-targets \
  --out Html \
  --out Stdout \
  --output-dir /coverage \
  --no-fail-fast \
  -- --nocapture 2>&1 | tee "$TARPAULIN_LOG"
TARPAULIN_STATUS=${PIPESTATUS[0]}
set +o pipefail

awk '
BEGIN {
  print "test | location | assert"
  print "---- | -------- | ------"
  pending = 0
  found = 0
}
{
  if (match($0, /thread '\''([^'\'']+)'\'' panicked at (.*)/, m)) {
    test_name = m[1]
    panic_detail = m[2]
    location = panic_detail

    if (match(panic_detail, /(.*):([0-9]+):([0-9]+):?$/, loc)) {
      location = loc[1] ":" loc[2]
    }

    assert_msg = "(assert text not found)"
    pending = 1
    next
  }

  if (pending == 1) {
    line = $0
    gsub(/^[[:space:]]+/, "", line)

    if (line == "" || line ~ /^note:/ || line ~ /^stack backtrace:/) {
      next
    }

    assert_msg = line
    print test_name " | " location " | " assert_msg
    found = 1
    pending = 0
  }
}
END {
  if (pending == 1) {
    print test_name " | " location " | " assert_msg
    found = 1
  }
  if (found == 0) {
    print "(none) | (none) | No failed tests detected"
  }
}
' "$TARPAULIN_LOG" > "$FAILED_REPORT"

echo "Wrote compact failure report: $FAILED_REPORT"

exit "$TARPAULIN_STATUS"
