#!/bin/sh
CC="${1:-./build/cc2}"
echo "=== libro tests ==="
cat src/main.cyr | "$CC" > /tmp/libro_test && chmod +x /tmp/libro_test && /tmp/libro_test
echo "exit: $?"
rm -f /tmp/libro_test
