#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../.." && pwd)"

check_no_matches() {
	local pattern="$1"
	local target="$2"

	if grep -RE --exclude='check-event-driven-drains.sh' "$pattern" "$target"; then
		echo "unexpected match for pattern: $pattern"
		echo "target: $target"
		exit 1
	fi
}

check_no_matches 'sleep\(Duration::from_millis\(10\)\)' \
	"$ROOT/rivetkit-rust/packages/rivetkit-core/src/actor/sleep.rs"
check_no_matches 'Mutex<Vec<JoinHandle' \
	"$ROOT/rivetkit-rust/packages/rivetkit-core/src/actor/sleep.rs"
check_no_matches 'begin_keep_awake|end_keep_awake|begin_internal_keep_awake|end_internal_keep_awake|begin_websocket_callback|end_websocket_callback' \
	"$ROOT/rivetkit-rust/packages/rivetkit-core"
