# Debugging Salt States

## If the state failed to render...

1. If developing with a cluster, SSH in to the master with `bolt ssh salt_master`.
2. Run `journalctl -u salt-master -n 1000`

## If the state failed on the minion...

1. If developing in a cluster or the failed minion is on a different machine, SSH in to the Minion with `bolt ssh name {my minion}`
2. Run `journalctl -u salt-minion -n 1000`
