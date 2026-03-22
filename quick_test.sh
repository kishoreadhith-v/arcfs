#!/usr/bin/env bash
set -euo pipefail

echo "quick_test.sh is now a wrapper for the full regression suite"
exec ./tests/regression_e2e.sh
