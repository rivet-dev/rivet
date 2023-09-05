# Developing

**The following should be installed in a dedicated VM for Rivet.**

## Prerequisites

-   Debian 11 (other Linux distros untested)
    -   Accessible from public IP
    -   Recommended: [Firewalls](/docs/getting_started/DEVELOPMENT_FIREWALLS.md)
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
nix-env -i git -i git-lfs
nix-shell -p git -p git-lfs --command "git clone https://github.com/rivet-gg/rivet.git /root/rivet"
```

> **Warp compatibility**
>
> Warp may have issues with the Nix installer since it does not use the default shell. [Read more.](https://docs.warp.dev/features/ssh)

## Step 3: Setup environment

Open your project's folder and run:

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

## Step 5: Boot the Rivet Hub

1. Clone the [Rivet Hub](https://github.com/rivet-gg/hub) with
2. Set `BASE_URL=https://{your base domain}` in the Hub's `.env`
3. Start the hub

## Step 6: Create a group

1. Open the hub (defaults to running at http://localhost:5080)
2. Register your account
3. Create a new group

## Step 7: Convert the group to a developer group

1. Copy the ID of your group from the URL
    - For example: `https://hub.rivet.gg/groups/d1f2e0b7-4c0d-48e1-8fae-309b98002b9f` would be `d1f2e0b7-4c0d-48e1-8fae-309b98002b9f`
2. Run `bolt admin team-dev create <GROUP_ID>`
    - Replace `<GROUP_ID>` with the ID you just copied

You should now see a `Developer` tab in the hub.

## Next steps

Read more about [working with Bolt](/docs/libraries/bolt/README.md) or see other [helpful docs](/README.md#-documentation-overview)
