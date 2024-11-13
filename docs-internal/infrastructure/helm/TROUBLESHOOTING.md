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
