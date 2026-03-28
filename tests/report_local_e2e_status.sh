#!/usr/bin/env bash
set -euo pipefail

CONTEXT="local/e2e-fuse"
LOG_FILE="/tmp/betterfs-local-e2e.out"

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Missing required command: $1" >&2
    exit 2
  fi
}

parse_repo_slug() {
  local remote_url
  remote_url="$(git config --get remote.origin.url || true)"

  if [[ -z "$remote_url" ]]; then
    echo "Could not read remote.origin.url" >&2
    exit 2
  fi

  if [[ "$remote_url" =~ ^https://github.com/([^/]+/[^/.]+)(\.git)?$ ]]; then
    echo "${BASH_REMATCH[1]}"
    return
  fi

  if [[ "$remote_url" =~ ^git@github.com:([^/]+/[^/.]+)(\.git)?$ ]]; then
    echo "${BASH_REMATCH[1]}"
    return
  fi

  echo "Unsupported remote URL format: $remote_url" >&2
  exit 2
}

post_status() {
  local repo="$1"
  local sha="$2"
  local state="$3"
  local description="$4"

  gh api \
    --method POST \
    "repos/${repo}/statuses/${sha}" \
    -f state="$state" \
    -f context="$CONTEXT" \
    -f description="$description" \
    -f target_url="https://github.com/${repo}/commit/${sha}" \
    >/dev/null
}

require_cmd gh
require_cmd git

if ! gh auth status >/dev/null 2>&1; then
  echo "GitHub CLI is not authenticated. Run: gh auth login" >&2
  exit 2
fi

repo_slug="$(parse_repo_slug)"
commit_sha="$(git rev-parse HEAD)"

echo "Running local E2E suite..."
if tests/regression_e2e.sh >"$LOG_FILE" 2>&1; then
  state="success"
else
  state="failure"
fi

pass_line="$(grep -E '^Pass:' "$LOG_FILE" | tail -n 1 || true)"
fail_line="$(grep -E '^Fail:' "$LOG_FILE" | tail -n 1 || true)"
result_line="$(grep -E '^RESULT:' "$LOG_FILE" | tail -n 1 || true)"

summary="${pass_line:-Pass: ?}, ${fail_line:-Fail: ?}, ${result_line:-RESULT: unknown}"
# Commit status description max is 140 chars.
description="${summary:0:140}"

post_status "$repo_slug" "$commit_sha" "$state" "$description"

echo "Posted GitHub status '${CONTEXT}' for ${commit_sha} (${state})."
echo "$summary"
echo "Log: $LOG_FILE"

if [[ "$state" == "failure" ]]; then
  exit 1
fi
