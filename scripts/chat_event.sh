#!/usr/bin/env bash
set -euo pipefail

# Usage: scripts/chat_event.sh <type:user|assistant> <user_said> <i_understood> [what_i_did=''] [path=dialog-dag.json]
# Example: scripts/chat_event.sh user "Build UL projection" "Capture request" ""

TYPE=${1:-}
SAID=${2:-}
UNDERSTOOD=${3:-}
DID=${4:-}
PATH_JSON=${5:-dialog-dag.json}

if [[ -z "$TYPE" || -z "$SAID" || -z "$UNDERSTOOD" ]]; then
  echo "Usage: $0 <type:user|assistant> <user_said> <i_understood> [what_i_did] [path]" >&2
  exit 2
fi

cargo run -q -p dialog_dag_tools --bin log_dialog_event -- "$PATH_JSON" "$TYPE" "$SAID" "$UNDERSTOOD" "${DID:-}" || {
  echo "Failed to log dialog event" >&2
  exit 1
}

