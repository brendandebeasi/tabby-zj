#!/usr/bin/env bash
# tabby-zj-hook.sh — Notification hook for tabby-zj Zellij plugin
#
# Sends pipe messages to the tabby-zj plugin to update sidebar indicators
# (busy, bell, input) in response to Claude Code and OpenCode events.
#
# Usage:
#   tabby-zj-hook.sh <EVENT>
#
# Supported events:
#   Claude Code: UserPromptSubmit, Stop, Notification
#   OpenCode:    start, complete, permission, question, error
#
# ── Claude Code setup (~/.claude/settings.json) ───────────────────────────
# {
#   "hooks": {
#     "UserPromptSubmit": [{"type": "command", "command": "/path/to/tabby-zj/scripts/tabby-zj-hook.sh UserPromptSubmit"}],
#     "Stop":             [{"type": "command", "command": "/path/to/tabby-zj/scripts/tabby-zj-hook.sh Stop"}],
#     "Notification":     [{"type": "command", "command": "/path/to/tabby-zj/scripts/tabby-zj-hook.sh Notification"}]
#   }
# }
#
# ── OpenCode setup (~/.config/opencode/opencode-notifier.json) ────────────
# {
#   "sound": false,
#   "notification": false,
#   "command": {
#     "enabled": true,
#     "path": "/path/to/tabby-zj/scripts/tabby-zj-hook.sh",
#     "args": ["{event}"]
#   }
# }

set -eu

EVENT="${1:-}"

if [ -z "$EVENT" ]; then
    echo "Usage: $(basename "$0") <event>" >&2
    exit 1
fi

# ── Zellij guard ──────────────────────────────────────────────────────────
# Skip silently if Zellij is not running. Two checks:
#   1. ZELLIJ_SESSION_NAME is set (we are inside a Zellij session)
#   2. `zellij list-sessions` returns at least one session (we are outside)

SESSION_NAME="${ZELLIJ_SESSION_NAME:-}"

if [ -z "$SESSION_NAME" ]; then
    if ! command -v zellij &>/dev/null; then
        exit 0
    fi
    SESSION_NAME=$(zellij list-sessions 2>/dev/null | awk 'NR==1 {print $1}' || true)
    if [ -z "$SESSION_NAME" ]; then
        exit 0
    fi
fi

# ── Pipe helper ───────────────────────────────────────────────────────────
# Sends a message to the tabby-zj plugin via `zellij pipe`.
# Uses ZELLIJ_SESSION_NAME if set (inside Zellij), otherwise targets a
# specific session by name (from outside Zellij).
send_pipe() {
    local msg="$1"
    if [ -n "${ZELLIJ_SESSION_NAME:-}" ]; then
        zellij pipe --plugin tabby-zj --name tabby -- "$msg" &>/dev/null || true
    else
        zellij --session "$SESSION_NAME" pipe --plugin tabby-zj --name tabby -- "$msg" &>/dev/null || true
    fi
}

# ── Parse JSON event ──────────────────────────────────────────────────────
# OpenCode may pass a JSON object as the event argument.
if [[ "$EVENT" =~ ^\{.*\}$ ]]; then
    PARSED=""
    if command -v jq &>/dev/null; then
        PARSED=$(printf '%s' "$EVENT" | jq -r '(.event // .type // .name // empty)' 2>/dev/null || true)
    elif command -v python3 &>/dev/null; then
        PARSED=$(python3 - <<'PYEOF' "$EVENT" 2>/dev/null || true)
import json, sys
try:
    data = json.loads(sys.argv[1])
    for key in ('event', 'type', 'name'):
        v = data.get(key)
        if isinstance(v, str) and v:
            print(v)
            break
except Exception:
    pass
PYEOF
    fi
    [ -n "$PARSED" ] && EVENT="$PARSED"
fi

# ── Event → indicator mapping ─────────────────────────────────────────────
case "$EVENT" in
    UserPromptSubmit|start|busy|working)
        # Agent started working — set busy, clear input prompt
        send_pipe "input:0"
        send_pipe "busy:1"
        ;;
    Stop|complete|done)
        # Agent finished — clear busy, ring bell
        send_pipe "busy:0"
        send_pipe "bell:1"
        ;;
    Notification|error|failed)
        # Async notification or error — ring bell
        send_pipe "bell:1"
        ;;
    permission|question)
        # Agent needs user input
        send_pipe "input:1"
        ;;
    subagent_complete)
        # Intermediate completion — no bell, just clear busy
        send_pipe "busy:0"
        ;;
    *)
        # Unknown event — no-op
        ;;
esac
