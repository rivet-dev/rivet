# Troubleshooting

## If the state failed to render...

1. If developing with a cluster, SSH in to the master with `bolt ssh salt_master`.
2. Run `journalctl -u salt-master -n 1000`

## If the state failed on the minion...

1. If developing in a cluster or the failed minion is on a different machine, SSH in to the Minion with `bolt ssh name {my minion}`
2. Run `journalctl -u salt-minion -n 1000`

## How to kill a stuck `salt.apply`?

1. SSH in to the minion with `bolt ssh name my-minion`.
2. Run `ps aux | grep salt-minion

```
root       20634  0.0  1.4 122488 28888 ?        Ss   01:27   0:00 /opt/saltstack/salt/bin/python3.10 /usr/bin/salt-minion
root       20642  0.1  4.6 794776 93824 ?        Sl   01:27   0:09 /opt/saltstack/salt/bin/python3.10 /usr/bin/salt-minion MultiMinionProcessManager MinionProcessManager
root       22660  0.1  6.9 835932 138440 ?       Sl   01:51   0:05 /opt/saltstack/salt/bin/python3.10 /usr/bin/salt-minion MultiMinionProcessManager MinionProcessManager ProcessPayload(jid=20230609015108483479) Minion._thread_return
root       25275  0.0  0.1   6240  2176 pts/0    S+   02:51   0:00 grep salt-minion
```

3. Kill the process containing `ProcessPayload(jid=20230609015108483479)` (in this example, 22660) with `kill 22660`

## What is my `salt.apply` stuck on?

Try a few things to figure this out:

-   Run `pstree -p my-pid` on the `salt-minion` process to see what subcommand is being ran
-   Read the `salt-minion` logs with `journalctl -u salt-minion`
-   Try applying specific SLS files with `salt apply 'my-minion' --sls my_file`

## Error when bootstrapping Minion: `RSA key format is not supported`

```bash
# Uninstall Salt
bolt ssh name staging-lnd-atl-crdb-05-2 'systemctl stop salt-minion; apt remove -y salt-cloud salt-common salt-minion; rm -rf /etc/salt /opt/saltstack /var/log/salt /var/cache/salt /run/salt /usr/bin/salt-*; echo Done'

# Re-run install_salt_minion
(cd infra/tf/pools && terraform state rm 'module.install_salt_minion["staging-lnd-atl-crdb-05-2"]')
bolt tf apply pools
```

