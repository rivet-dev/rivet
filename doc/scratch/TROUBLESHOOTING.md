# Troubleshooting

## `version `GLIBC_2.32' not found`

See [here](https://github.com/oxalica/rust-overlay/issues/54#issuecomment-985486467).

TLDR, run this:

```
rm -rf ~/.cargo/bin
```

## Local cached Docker images disappearing

To force all the images to be pulled again, remove the Bolt cache with `rm .bolt-meta.json` and run `bolt up` again.

If using NixOS, make sure the [Docker auto pruning](https://search.nixos.org/options?channel=22.05&show=virtualisation.docker.autoPrune.enable&from=0&size=50&sort=relevance&type=packages&query=virtualisation.docker.autoPrune.enable) is disabled.
