# Rivet Cloud CLI

A premium command-line interface for deploying Docker images and streaming logs on [Rivet Cloud](https://hub.rivet.dev).

## Installation

```bash
# From the monorepo root
cd cloud-cli
bun install
bun run build

# Run directly with Bun
bun run src/index.ts
```

## Authentication

All commands require a Rivet Cloud API token. Get yours from the [Rivet dashboard](https://hub.rivet.dev) → project → **Connect** → **Rivet Cloud**.

Set it as an environment variable (recommended):

```bash
export RIVET_CLOUD_TOKEN=cloud_api_...
```

Or pass it per-command:

```bash
rivet-cloud deploy --token cloud_api_...
```

---

## Commands

### `rivet-cloud deploy`

Build a Docker image and deploy it to a Rivet Cloud managed pool.

```bash
rivet-cloud deploy [options]
```

| Option | Description | Default |
|---|---|---|
| `-t, --token <token>` | Cloud API token | `RIVET_CLOUD_TOKEN` |
| `-n, --namespace <name>` | Target namespace (created if absent) | `production` |
| `-p, --pool <name>` | Managed pool name | `default` |
| `--context <path>` | Docker build context directory | `.` |
| `-f, --dockerfile <path>` | Path to Dockerfile | auto-detect |
| `--tag <tag>` | Docker image tag | git short SHA or timestamp |
| `--min-count <n>` | Minimum runner instances | `1` |
| `--max-count <n>` | Maximum runner instances | `5` |
| `-e, --env <KEY=VALUE>` | Environment variable (repeatable) | — |
| `--command <cmd>` | Override container entrypoint | — |
| `--args <args>` | Space-separated args for the command | — |
| `--platform <platform>` | Docker build platform | `linux/amd64` |
| `--api-url <url>` | Cloud API base URL | `https://cloud-api.rivet.dev` |

**Examples:**

```bash
# Deploy with defaults (builds . → namespace "production" → pool "default")
rivet-cloud deploy

# Deploy to a PR preview namespace
rivet-cloud deploy --namespace pr-42

# Deploy with environment variables and custom counts
rivet-cloud deploy \
  --namespace staging \
  --min-count 2 \
  --max-count 10 \
  --env DATABASE_URL=postgres://... \
  --env API_KEY=secret
```

---

### `rivet-cloud logs`

Stream real-time logs from a Rivet Cloud managed pool (similar to the hub.rivet.dev Logs view).

```bash
rivet-cloud logs [options]
```

| Option | Description | Default |
|---|---|---|
| `-t, --token <token>` | Cloud API token | `RIVET_CLOUD_TOKEN` |
| `-n, --namespace <name>` | Target namespace | `production` |
| `-p, --pool <name>` | Managed pool name | `default` |
| `--filter <text>` | Only show lines containing this string | — |
| `--region <region>` | Filter by region slug (e.g. `us-west-1`) | all regions |
| `--api-url <url>` | Cloud API base URL | `https://cloud-api.rivet.dev` |

Logs stream over SSE with automatic reconnection (up to 8 retries with exponential back-off). Press **Ctrl+C** to stop.

**Examples:**

```bash
# Stream all logs
rivet-cloud logs

# Filter to error lines in us-west-1
rivet-cloud logs --filter ERROR --region us-west-1

# Follow a PR preview namespace
rivet-cloud logs --namespace pr-42
```

---

## Global Flags

| Flag | Description |
|---|---|
| `-h, --help` | Show help for any command |
| `-V, --version` | Output CLI version |

---

## Design Notes

- **Authentication** — uses `RIVET_CLOUD_TOKEN` (the same secret as `rivet-dev/deploy-action`)
- **Deploy flow** — inspects token → ensures namespace → fetches registry credentials → `docker build` + `docker push` → upserts managed pool
- **Pool name** — defaults to `"default"` matching the deploy-action convention
- **Log streaming** — SSE stream with exponential back-off reconnect, mirrors `use-deployment-logs-stream.ts` in the frontend
- **Colors** — Rivet brand palette (`#FF4500` accent, `#FAFAFA` primary, `#A0A0A0` secondary)
- **Progress** — uses [tasuku](https://github.com/privatenumber/tasuku) for deploy task indication

## Development

```bash
# Run tests
bun test

# Type-check
bun run check-types

# Run CLI directly
bun run src/index.ts --help
```
