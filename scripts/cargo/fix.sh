set -e

# Clean svc and libs
(cd svc && cargo fix --workspace --allow-dirty)
(cd lib/bolt && cargo fix --workspace --allow-dirty)
(cd lib/convert && cargo fix --workspace --allow-dirty)
(cd lib/util && cargo fix --workspace --allow-dirty)

# Format svc and libs
(cd svc && cargo fmt)
(cd lib/bolt && cargo fmt)
(cd lib/convert && cargo fmt)
(cd lib/util && cargo fmt)
