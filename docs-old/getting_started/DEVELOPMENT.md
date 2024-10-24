# Development

> [!IMPORTANT]
>
> **WIP Open-source Developer Experience**
>
> We encourage developers to use the self-hosted version of Rivet, but we're still working on improving the
> open source developer experience.
>
> Once running, Rivet itself is very lightweight since it's all Rust.
>
> Relevant issues:
>
> - <https://github.com/rivet-gg/rivet/issues/658>
> - <https://github.com/rivet-gg/rivet/issues/657>
> - <https://github.com/rivet-gg/rivet/issues/656>
> - <https://github.com/rivet-gg/rivet/issues/655>
> - <https://github.com/rivet-gg/rivet/issues/654>
> - <https://github.com/rivet-gg/rivet/issues/652>
> - <https://github.com/rivet-gg/rivet/issues/651>

## Overview

There are two ways to run Rivet:

- [Dev Container](#method-1-dev-container)
- [Virtual Machine](#method-2-virtual-machine)

Once you have set up the fundamentals, you'll follow the [common steps](#common-steps).

## Method 1: Dev Container

This method is recommended if testing Rivet locally or actively developing on the cluster.

Dev containers also allow you to use [GitHub Codespaces](https://github.com/features/codespaces) in order to
develop without installing anything locally.

### Prerequisites

- Visual Studio Code

> [!NOTE]
>
> **Windows Users**
>
> You'll also [need WSL installed](https://learn.microsoft.com/en-us/windows/wsl/install). Because the default
> settings only give 16GB of ram to WSL, you'll need to
> [change the WSL config](https://learn.microsoft.com/en-us/windows/wsl/wsl-config), and allocate at least
> 24GB of ram to WSL. This is needed for the Rivet's source code to build without running out of memory.

> [!NOTE]
>
> **Optionally using GitHub Codespaces**
>
> At this point, you can either run a dev containers locally, or set up a GitHub Codespace. Codespaces cost
> money to use, but are a zero-configuration setup in case you might not have good hardware to use. You can
> start a Codespace from Rivet's main repo, though you will need to configure it so that it has at least 32GB
> of ram.
>
> The rest of this section will assist you in setting up a dev container locally.

### Step 1: Install dependencies

- **[Docker](https://docs.docker.com/engine/install/)** See [here](https://docs.docker.com/engine/install/)
  for install instructions.
- **[Git](https://git-scm.com/)** See [here](https://git-scm.com/book/en/v2/Getting-Started-Installing-Git)
  for install instructions.

### Step 2: Set up Dev Container

Dev containers are a feature of Visual Studio Code, so you won't be able to use them in other editors. To set
up your editor to work with dev containers, you can follow
[this official guide](https://code.visualstudio.com/docs/devcontainers/containers)

### Step 3: Clone repository

We need to clone the repository for the dev container configuration to work. Run:

```sh
git clone https://github.com/rivet-gg/rivet.git
```

> [!IMPORTANT]
>
> **Windows Patch**
>
> If you're on Windows, you'll need to run a script that will set up symlinks properly. This command needs to
> be run from inside WSL, or inside the dev container. Run:
>
> ```sh
> rm -rf "infra/helm/clickhouse/charts"
> ln -s ../charts "infra/helm/clickhouse/charts"
>
> rm -rf "infra/helm/redis/charts"
> ln -s ../charts "infra/helm/redis/charts"
>
> rm -rf "infra/helm/redis-cluster/charts"
> ln -s ../charts "infra/helm/redis-cluster/charts"
> ```

### Step 4: Open in Dev Container

Open the repository in Visual Studio Code. You should see a prompt to "Reopen in Container". Click this to
start the dev container. If you don't see this prompt, you can open the command palette (Ctrl+Shift+P or
Cmd+Shift+P on Mac) and run and run "Remote-Containers: Reopen in Container".

You can now skip to the [Common steps](#common-steps) section.

### Step 5: Setup dev tunnel (optional)

Rivet needs a publicly accessible IP in order to be able to deploy servers. Without it, you can still run
Rivet, but you won't be able to access servers.

Read the guide on setting up a dev tunnel (similar to ngrok) [here](/docs/infrastructure/dev-tunnel/SETUP.md).

## Method 2: Virtual Machine

This is best if running a small deployment of Rivet on a cloud provider.

### Prerequisites

**The following should be installed in a dedicated VM for Rivet.**

- Debian 11 (other Linux distros untested)
  - Accessible from public IP
  - Recommended: [Firewalls](/docs/getting_started/DEVELOPMENT_FIREWALLS.md)

### Step 1: Install dependencies

- **[Docker](https://docs.docker.com/engine/install/)** See [here](https://docs.docker.com/engine/install/)
  for install instructions.
- **[Nix package manager](https://nixos.org/download.html)** Nix is required to set up the development
  environment.
  - ```sh
    sh <(curl -L https://nixos.org/nix/install) --daemon
    ```
- **[Git](https://git-scm.com/)** See [here](https://git-scm.com/book/en/v2/Getting-Started-Installing-Git)
  for install instructions.

### Step 2: Clone repository

Run:

```sh
git clone https://github.com/rivet-gg/rivet.git
```

## Common steps

Now that you have the environment set up either in a dev container or VM, we can start setting up the Rivet
cluster.

> **Warp compatibility**
>
> Warp may have issues with the Nix installer since it does not use the default shell.
> [Read more.](https://docs.warp.dev/features/ssh)

### Step 1: Boot cluster

Run:

```sh
nix-shell --run "bolt init dev --yes"
```

This will run a series of scripts to set up the required Kubernetes cluster, required dependencies, and deploy
Rivet to the cluster.

Run this command any time you update to a new version of Rivet.

> [!TIP]
>
> See the `namespaces/dev.toml` and `secrets/dev.toml` file to see the generated namespace configs.

> [!IMPORTANT]
>
> **Configure Public IP**
>
> If you're running Rivet on a virtual machine with a public IP, configure the IP in `namespaces/dev.toml`.
>
> For example:
>
> ```toml
> [cluster.single_node]
> public_ip = "1.2.3.4"
> ```
>
> Then run again:
>
> ```sh
> nix-shell --run "bolt init dev --yes"
> ```
>
> Ngrok support coming soon to simplify this for local development:
> <https://github.com/rivet-gg/rivet/issues/659>

### Step 2: Boot the Rivet Hub

Now, you have to start the hub frontend.

1. Clone the [Rivet Hub](https://github.com/rivet-gg/hub) with
2. Create a file called `.env` in the root of the `hub` repo
   - Set `RIVET_API_ENDPOINT=http://localhost:8080` in the `.env` file
3. Start the hub
4. Visit <http://localhost:5080>
5. Register an account with your email

### Step 3: Log in to cluster dashboard

```sh
bolt admin login
```

This will provide you a URL to authenticate as an admin to the cluster. If you get logged out, run this
command again to sign in again.

### Next steps

You now have a standalone instance of Rivet running.

This currently does not create & run game servers. This requires further tokens & configuration.

Read more about [working with Bolt](/docs/libraries/bolt/README.md) or see other
[helpful docs](/README.md#-documentation-overview)
