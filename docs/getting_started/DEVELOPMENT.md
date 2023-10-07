# Developing

**The following should be installed in a dedicated VM for Rivet.**

## Prerequisites

-   Debian 11 (other Linux distros untested)
    -   Accessible from public IP
    -   Recommended: [Firewalls](/docs/getting_started/DEVELOPMENT_FIREWALLS.md)
-   [Cloudflare website](https://developers.cloudflare.com/fundamentals/get-started/setup/add-site/) (free)
-   [Linode account](https://login.linode.com/signup) (more providers coming soon)

## Step 1: Install dependencies

### [Docker](https://docs.docker.com/engine/install/)

_Docker is required to run Rivet services._

See [here](https://docs.docker.com/engine/install/) for install instructions.

### [Nix package manager](https://nixos.org/download.html)

_Nix is required to set up the development environment._

Run:

```
sh <(curl -L https://nixos.org/nix/install) --daemon
```

### [Git](https://git-scm.com/)

See [here](https://git-scm.com/book/en/v2/Getting-Started-Installing-Git) for install instructions.

## Step 2: Clone repository

Run:

```
git clone https://github.com/rivet-gg/rivet.git
```

> **Warp compatibility**
>
> Warp may have issues with the Nix installer since it does not use the default shell. [Read more.](https://docs.warp.dev/features/ssh)

## Step 3: Boot cluster

Run:

```
nix-shell --run "bolt init dev --yes"
```

This will:

1. Prompt you for parameters to generate your cluster's config
2. Provision required infrastructure for the cluster

Run this command any time you update to a new version of Rivet.

> **Tip**
>
> See the `namespaces/dev.toml` and `secrets/dev.toml` file to see the generated namespace configs.

## Step 4: Boot the Rivet Hub

1. Clone the [Rivet Hub](https://github.com/rivet-gg/hub) with
2. Set `BASE_URL=https://{your base domain}` in the Hub's `.env`
3. Start the hub

## Step 5: Create a group

1. Open the hub (defaults to running at http://localhost:5080)
2. Register your account
3. Create a new group

## Step 6: Convert the group to a developer group

1. Copy the ID of your group from the URL
    - For example: `https://hub.rivet.gg/groups/d1f2e0b7-4c0d-48e1-8fae-309b98002b9f` would be `d1f2e0b7-4c0d-48e1-8fae-309b98002b9f`
2. Run `nix-shell --run "bolt admin team-dev create <GROUP_ID>"`
    - Replace `<GROUP_ID>` with the ID you just copied

You should now see a `Developer` tab in the hub.

## Next steps

Read more about [working with Bolt](/docs/libraries/bolt/README.md) or see other [helpful docs](/README.md#-documentation-overview)
