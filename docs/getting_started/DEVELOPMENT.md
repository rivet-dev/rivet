# Developing

> 🚨 **READ BEFORE PROCEEDING** 🚨
>
> We encourage developers to use the self-hosted version of Rivet, but we're
> still working on improving the self-hosted developer experience.
>
> At the moment, all the services need to be built from scratch and deployed
> to a fully loaded Kubernetes cluster, which requires a beefy Linux machine.
> (Ideally 8 CPU cores, 32 GB RAM to build the required components for the
> cluster & run Rust Analyzer at the same time.)
>
> Once running, Rivet itself is very lightweight since it's all Rust. If you
> want to help make Rivet easier to run, please reach out and we can help
> provide guidance on [Discord](https://discord.gg/BG2vqsJczH) when
> implementing the following issues:
>
> -   <https://github.com/rivet-gg/rivet/issues/154>
> -   <https://github.com/rivet-gg/rivet/issues/156>
> -   <https://github.com/rivet-gg/rivet/issues/157>

There are two methods to set up a development environment:

-   Using Devcontainers/Codespaces ([instructions](#dev-container))
-   Directly on a virtual machine ([instructions](#virtual-machine))

Devcontainers are the primary choice as they're more supported by the
Rivet team. This is because their setup is consistant across Linux, MacOS, and
Windows, since devcontainers run inside of a Docker container. The Devcontainer
spec also allows you to use [GitHub
Codespaces](https://github.com/features/codespaces) if that is easier.

## Dev Container

### Prerequisites

-   Visual Studio Code

**Windows Note**: If you're on Windows, you'll also [need WSL
installed](https://learn.microsoft.com/en-us/windows/wsl/install). Because the
default settings only give 16GB of ram to WSL, you'll need to [change the WSL
config](https://learn.microsoft.com/en-us/windows/wsl/wsl-config), and allocate
at least 24GB of ram to WSL. This is needed for the Rivet's source code to build
without running out of memory.

**Codespaces Notes** At this point, you can either run a Devcontainer locally, or set up a GitHub
Codespace. Codespaces cost money to use, but are a zero-configuration
setup in case you might not have good hardware to use. You can start a Codespace from Rivet's main
repo, though you will need to configure it so that it has at least 32GB of ram.

The rest of this section will assist you in setting up a Devcontainer locally.

### Step 1: Install dependencies

#### [Docker](https://docs.docker.com/engine/install/)

_Docker is required to run Rivet services._

See [here](https://docs.docker.com/engine/install/) for install instructions.

### Step 2: Set up Devcontainers

Devcontainers are a feature of Visual Studio Code, so you won't be able to use
them in other editors. To set up your editor to work with Devcontainers, you can
follow [this official
guide](https://code.visualstudio.com/docs/devcontainers/containers)

### Step 3: Clone repository

We need to clone the repository for the Devcontainer configuration to work. Run:

```sh
git clone https://github.com/rivet-gg/rivet.git
```

#### [Git](https://git-scm.com/)

See [here](https://git-scm.com/book/en/v2/Getting-Started-Installing-Git) for
install instructions.

**Windows Note**: If you're on Windows, you'll need to run a script that will
set up symlinks properly. This command needs to be run from inside WSL, or
inside the Devcontainer. Run:

```sh
rm -rf "infra/helm/clickhouse/charts"
ln -s ../charts "infra/helm/clickhouse/charts"

rm -rf "infra/helm/redis/charts"
ln -s ../charts "infra/helm/redis/charts"

rm -rf "infra/helm/redis-cluster/charts"
ln -s ../charts "infra/helm/redis-cluster/charts"
```

### Step 4: Open in Devcontainer

Open the repository in Visual Studio Code. You should see a prompt to "Reopen in
Container". Click this to start the Devcontainer. If you don't see this prompt,
you can open the command palette (Ctrl+Shift+P or Cmd+Shift+P on Mac) and run
and run "Remote-Containers: Reopen in Container".

You can now skip to the [Common steps](#common-steps) section.

## Virtual Machine

### Prerequisites

**The following should be installed in a dedicated VM for Rivet.**

-   Debian 11 (other Linux distros untested)
    -   Accessible from public IP
    -   Recommended: [Firewalls](/docs/getting_started/DEVELOPMENT_FIREWALLS.md)

### Step 1: Install dependencies

#### [Docker](https://docs.docker.com/engine/install/)

_Docker is required to run Rivet services._

See [here](https://docs.docker.com/engine/install/) for install instructions.

#### [Nix package manager](https://nixos.org/download.html)

_Nix is required to set up the development environment._

Run:

```sh
sh <(curl -L https://nixos.org/nix/install) --daemon
```

### Step 2: Clone repository

Run:

```sh
git clone https://github.com/rivet-gg/rivet.git
```

#### [Git](https://git-scm.com/)

See [here](https://git-scm.com/book/en/v2/Getting-Started-Installing-Git) for install instructions.

## Common steps

Now that you have the environment set up either in a Devcontainer or VM, we can
start setting up the Rivet cluster.

> **Warp compatibility**
>
> Warp may have issues with the Nix installer since it does not use the default shell. [Read more.](https://docs.warp.dev/features/ssh)

### Step 1: Boot cluster

Run:

```sh
nix-shell --run "bolt init dev --yes"
```

This will:

1. Prompt you for parameters to generate your cluster's config
2. Provision required infrastructure for the cluster

Run this command any time you update to a new version of Rivet.

> **Tip**
>
> See the `namespaces/dev.toml` and `secrets/dev.toml` file to see the generated namespace configs.

### Step 2: Boot the Rivet Hub

Now, you have to start the hub frontend.

1. Clone the [Rivet Hub](https://github.com/rivet-gg/hub) with
2. Create a file called `.env` in the root of the `hub` repo
    - Set `RIVET_API_ENDPOINT=http://localhost:8080` in the `.env` file
3. Start the hub
4. Visit <http://localhost:5080>
5. Register an account with your email

### Step 3: Promote yourself to admin

```sh
nix-shell --run "bolt db sh db-user --query 'UPDATE users SET is_admin = true'"
```

You may need to clear the local cache for this change to appear. ([Related issue](https://github.com/rivet-gg/rivet/issues/152))

_This command sets all users to admin. We're assuming you're the only user in the cluster. Do not run this command again._

### Step 4: Create a developer group

You should now see a "Create Group" button on the hub. Proceed to create a group and start developing.

### Next steps

Read more about [working with Bolt](/docs/libraries/bolt/README.md) or see other [helpful docs](/README.md#-documentation-overview)
