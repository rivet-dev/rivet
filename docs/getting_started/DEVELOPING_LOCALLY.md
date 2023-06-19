# Developing Locally

**The following should be installed in a dedicated VM for Rivet.**

## Prerequisites

-   Debian 11 (other Linux distros untested)
    -   Accessible from public IP
    -   Recommended: [Firewalls](/docs/getting_started/DEVELOPING_LOCALLY_FIREWALLS.md)
-   [Cloudflare website](https://developers.cloudflare.com/fundamentals/get-started/setup/add-site/) (free)
-   [Linode account](https://login.linode.com/signup) (more providers coming soon)

## Step 1: Install [Nix package manager](https://nixos.org/download.html)

Nix is required to set up the development environment.

Run:

```
sh <(curl -L https://nixos.org/nix/install) --daemon
```

## Step 2: Clone repository

This will use Nix to install Git, then clone the repository to `/root/rivet`. You can clone the repository any place you like.

Run:

```
nix-shell -p git -p git-lfs --command "git clone https://github.com/rivet-gg/rivet.git /root/rivet"
```

## Step 3: Setup environment

Run:

```
nix-shell
./scripts/setup.sh
```

_Run `nix-shell` in every new shell you create if you're not using [Lorri](/docs/infrastructure/nix/LORRI.md)._

## Step 4: Initiate new namespace

Run:

```
bolt init dev
```

This will:

1. Prompt you for parameters to generate your cluster's config
2. Provision required infrastructure for the cluster

Run this command any time you update to a new version of Rivet.

> **Tip**
>
> See the `namespaces/dev.toml` and `secrets/dev.toml` file to see the generated namespace configs.

> **Tip**
>
> You can create multiple namespaces with different configs. The active namespace is set in the `Bolt.local.toml` file. We use the name `dev` as a safe default.

## Next steps

Read more about [working with Bolt](/docs/libraries/bolt/README.md) or see other [helpful docs](/README.md#-documentation-overview)
