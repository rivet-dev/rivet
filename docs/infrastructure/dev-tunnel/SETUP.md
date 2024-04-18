# Setup Dev Tunnel

This guide will show you how to set up a dev tunnel (similar to [ngrok](https://ngrok.com/)) for developing Rivet locally.

This will run a Terraform plan to deploy two components:

-   A server on Linode that will forward traffic to your local machine
-   A Docker container that will connect to the remote server over SSH and expose a reverse tunnel

## Prerequisites

Make sure to run `nix-shell` for all subsequent commands.

-   Docker
-   Linode API Key

## Step 1: Create Dev Tunnel

```sh
task dev-tunnel:up
```

This will prompt you to past your Linode API token.

Once complete, this will print an IP to your console like:

```toml
ip = "1.2.3.4"
```

## Step 2: Update public IP

Open your namespace config in `namespaces/dev.toml`.

-   Update `cluter.single_node.public_ip` to the IP from the last step. By default, the config is generated with `public_ip = "127.0.0.1"`.
-   If exists, delete the line that says `api_http_port = 8080`.
-   Validate that there are no ports overridden (i.e. `cluter.single_node.api_http_port`).

If you need your IP again, run `task dev-tunnel:get-ip`.

## Step 3: Update infrastructure

To deploy the new DNS records & configs, run:

```sh
bolt infra up
```

## Step 4: Valdiate deployment

Validate you can reach your local server on the public IP, replace `MY_TUNNEL_IP` with the IP from the last step:

```sh
curl MY_TUNNEL_IP:80
```

This should return a 404 response:

```
404 page not found
```

This means your server is now accessible.

If you have DNS configured, you should be able to reach your server from `api.my

## Cleanup

```sh
task dev-tunnel:down
```
