#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# Legacy alias: historically used as the "final" demo entrypoint.
# Keep compatibility by forwarding to the review demo.
exec bash "$SCRIPT_DIR/demo_review.sh" "$@"
