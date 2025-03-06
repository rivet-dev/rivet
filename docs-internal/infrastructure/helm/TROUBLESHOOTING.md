# Troubleshooting

## `Error: cannot re-use a name that is still in use`

First, try uninstalling the chart manually with:

```
helm uninstall -n foo bar
```

If that doesn't work, check if there is a secret named something like `sh.helm.release.v1.bar.v1` in the given
namespace:

```
kubectl get secret -n foo
```

If so, delete the secret with:

```
kubectl delete secret -n foo foobar
```

Then re-run the Terraform plan.

## `Error: another operation (install/upgrade/rollback) is in progress`

Your helm deployment probabily failed. You need to roll it back. For example:

Roll back to the last deployed revision:

```sh
[nix-shell:/home/rivet/rivet-ee]# helm list -n promtail --all
NAME            NAMESPACE       REVISION        UPDATED                                 STATUS          CHART           APP VERSION
promtail        promtail        8               2025-03-06 01:17:04.451599765 +0000 UTC pending-upgrade promtail-6.15.1 2.8.4

[nix-shell:/home/rivet/rivet-ee]# helm -n promtail history promtail
REVISION        UPDATED                         STATUS          CHART           APP VERSION     DESCRIPTION
1               Sun Oct 29 18:41:08 2023        superseded      promtail-6.15.1 2.8.4           Install complete
2               Mon Nov  6 06:47:58 2023        superseded      promtail-6.15.1 2.8.4           Upgrade complete
3               Sun Feb  4 21:23:30 2024        superseded      promtail-6.15.1 2.8.4           Upgrade complete
4               Mon Feb  5 19:57:21 2024        superseded      promtail-6.15.1 2.8.4           Upgrade complete
5               Sat Feb 24 21:32:04 2024        superseded      promtail-6.15.1 2.8.4           Upgrade complete
6               Mon May 20 23:57:18 2024        deployed        promtail-6.15.1 2.8.4           Upgrade complete
7               Thu Mar  6 01:11:20 2025        failed          promtail-6.15.1 2.8.4           Upgrade "promtail" failed: context deadline exceeded
8               Thu Mar  6 01:17:04 2025        pending-upgrade promtail-6.15.1 2.8.4           Preparing upgrade

[nix-shell:/home/rivet/rivet-ee]# helm -n promtail rollback promtail 6
Rollback was a success! Happy Helming!
```

Apply your Helm chart again:

```sh
[nix-shell:/home/rivet/rivet-ee]# helm list -n promtail
NAME            NAMESPACE       REVISION        UPDATED                                 STATUS          CHART           APP VERSION
promtail        promtail        9               2025-03-06 01:28:52.820014049 +0000 UTC deployed        promtail-6.15.1 2.8.4
```
