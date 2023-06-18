# Setup Guide

**The following should be installed in a dedicated VM for Rivet.**

### Prerequisites

-   Debian 11 (Ubuntu untested)
    -   Accessible from public IP
    -   [Firewalls](./doc/DEV_FIREWALLS.md)
-   [Nix package manager](https://nixos.org/download.html) (needed to setup dev environment)
    -   TLDR: `sh <(curl -L https://nixos.org/nix/install) --daemon`
-   [Git](https://git-scm.com/) & [Git LFS](https://git-lfs.com/) (needed to clone source code)
    -   TLDR: `nix-env -i git git-lfs` (must have Nix installed first)
-   [lorri + direnv](https://github.com/nix-community/lorri) (optional, will prevent the need to run `nix-shell` every new shell)

### Step 1: Setup environment

```
nix-shell
direnv allow  # If using Lorri (optional)
./scripts/setup.sh
```

_Run `nix-shell` in every new shell you create if you're not using [Lorri](https://github.com/nix-community/lorri)._

### Step 2: Initiate new namespace

Run the following:

```
bolt init prod
```

This is going to prompt you for configuration parameters to set up a namespace called `prod`, then create the required infrastructure.

Run this command any time you update to a new version of Rivet.

> **Tip**
>
> See the `namespaces/prod.toml` and `secrets/prod.toml` file to see the generated namespace configs.

> **Tip**
>
> You can create multiple namespaces. The active namespace is set in the `Bolt.local.toml` file.

### Next steps

Read more about [working with Bolt](/docs/libraries/bolt/README.md).
