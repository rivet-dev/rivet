set -e

(cd svc && cargo check --workspace)
(cd lib/bolt && cargo check --workspace)
(cd lib/convert && cargo check --workspace)
(cd lib/util && cargo check --workspace)
