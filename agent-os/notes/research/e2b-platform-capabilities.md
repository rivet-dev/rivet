# E2B Platform Capabilities Research

Research date: 2026-03-28
Source: e2b.dev documentation (e2b.mintlify.app), GitHub (e2b-dev/infra), blog posts

---

## 1. Sandbox / Isolation Capabilities

**Technology**: Firecracker microVMs (the same technology AWS Lambda uses).

- Each sandbox is an isolated Linux microVM, not a container
- Firecracker boots in ~125ms, with <5MB RAM overhead per microVM
- Firecracker VMM is ~50k lines of Rust (vs QEMU's ~2M lines of C), minimizing attack surface
- Runs a Linux LTS 6.1 kernel (locked at template build time; cannot upgrade without rebuilding the template)
- Full Ubuntu environment (default: Ubuntu 22.04 for desktop use cases)
- Hardware-level isolation via KVM -- each sandbox gets its own kernel, not shared with others
- Huge Pages enabled by default (2 MiB chunks instead of 4 KiB) for up to 5x faster memory-intensive startup

**Lifecycle**:
- Max continuous runtime: 1 hour (Hobby), 24 hours (Pro), custom (Enterprise)
- Runtime limit resets after pause/resume cycles
- States: Running -> Paused -> Snapshotting -> Killed (terminal)
- Pausing takes ~4 seconds per 1 GiB RAM; resuming takes ~1 second
- Paused sandboxes persist indefinitely (no automatic deletion)
- Auto-pause on timeout supported; auto-resume on activity (SDK calls or HTTP traffic)

---

## 2. Filesystem Access and Persistence

**Ephemeral Filesystem (per-sandbox)**:
- 10 GB disk (Hobby), 20 GB disk (Pro)
- Full Linux filesystem inside the microVM
- SDK methods: `files.read()`, `files.write()`, `files.write()` with arrays for batch writes
- Directory listing, stat/info, move, remove, mkdir operations
- File upload/download via SDK (with signed URL support)
- File watching: `files.watchDir()` with recursive option, supports CREATE and WRITE events
  - Caveat: events are async; rapid folder creation may miss non-CREATE events

**Persistent Volumes (beta)**:
- Independent of sandbox lifecycle -- data survives sandbox shutdown
- Can be mounted to multiple sandboxes simultaneously
- SDK operations for read/write/upload/download even when not mounted to an active sandbox
- Currently in private beta (requires contacting support@e2b.dev)

**Cloud Storage Integration**:
- GCS, S3, and Cloudflare R2 buckets mountable via FUSE
- Requires custom template with gcsfuse or s3fs pre-installed
- Mounted at runtime, not at template build time

---

## 3. Process Execution

**Commands API** (`sandbox.commands`):
- `commands.run(cmd)` -- synchronous execution, returns result with stdout/stderr
- `commands.run(cmd, { background: true })` -- non-blocking, returns handle
- Background process handle supports: `kill()`, `wait()`, output iteration/callbacks
- Streaming: `onStdout` / `onStderr` callbacks for real-time output
- Per-command environment variables (override global sandbox env vars)
- Per-command working directory
- Per-command timeout

**PTY Support** (`sandbox.pty`):
- Full pseudo-terminal with `TERM=xterm-256color`
- ANSI colors and escape sequences supported
- Bidirectional: `sendInput()` for stdin, callback for stdout
- Configurable dimensions (cols/rows), dynamically resizable
- Session persistence: PTY sessions survive disconnect, reconnectable with new data handler
- Default timeout 60s; set to 0 for indefinite

**Process Management**:
- List running processes
- Connect to existing processes by ID
- Send signals to processes
- Send stdin input, close stdin
- Stream input to processes

---

## 4. Networking

**Outbound**:
- Internet access enabled by default (can disable with `allowInternetAccess: false`)
- Fine-grained network rules: allow/deny lists for IPs, CIDR blocks, and domains
- Domain filtering works for HTTP (port 80) and TLS (port 443) only, via Host header / SNI inspection
- Wildcard subdomains supported (e.g., `*.mydomain.com`)
- DNS auto-resolves to 8.8.8.8 when domain rules are configured
- UDP protocols (QUIC/HTTP3) not supported for domain filtering
- Max 2,500 outbound connections per sandbox
- IP tunneling via Shadowsocks proxy for dedicated outbound IPs

**Inbound / Service Exposure**:
- Every sandbox gets a public HTTPS URL: `https://[PORT]-[SANDBOX_ID].e2b.app`
- `sandbox.getHost(port)` returns the external hostname for a given port
- Optional traffic restriction: `allowPublicTraffic: false` requires `e2b-traffic-access-token` header
- `maskRequestHost` option to customize Host header forwarded to sandbox services
- Custom domain support via Caddy reverse proxy setup

**SSH Access**:
- Via WebSocket proxy (websocat -> OpenSSH server on port 22)
- Requires custom template with openssh-server and websocat installed
- Supports SCP/SFTP file transfers

---

## 5. Session Management

**Sandbox Identification**:
- Each sandbox has a unique `sandboxId`
- Arbitrary key-value metadata can be attached at creation
- Sandboxes can be listed, filtered by state and metadata

**Connection / Reconnection**:
- `Sandbox.connect(sandboxId)` reconnects to an existing running sandbox
- SDK automatically resumes paused sandboxes on connect (with auto-resume enabled)
- `Sandbox.list()` returns paginated list of sandboxes with their state and metadata

**Lifecycle Events**:
- Pull-based API: `GET /events/sandboxes/{sandboxId}` with offset/limit pagination
- Push-based webhooks for: created, killed, updated, paused, resumed, checkpointed
- Webhook payloads include sandbox metadata, execution details (vCPU, memory, duration)
- HMAC-SHA256 signature verification on webhooks

**Lifecycle Info**:
- `getInfo()` returns: sandbox ID, template ID, creation timestamp, projected end time
- `getMetrics()` returns: CPU usage %, core count, memory used/total, disk used/total (sampled every 5s)

---

## 6. State / Persistence / Snapshots / Pausing

**Pause/Resume**:
- Saves complete state: filesystem + memory (including running processes, variables, data)
- One-to-one: a paused sandbox resumes to the same instance
- Paused sandboxes persist indefinitely
- ~4s/GiB to pause, ~1s to resume
- Runtime limit resets after resume

**Snapshots**:
- Captures complete state (filesystem + memory) at a point in time
- Original sandbox briefly pauses during snapshot, then continues running
- One-to-many: a single snapshot can create multiple new sandboxes
- Create via `sandbox.createSnapshot()` or `Sandbox.createSnapshot(sandboxId)`
- Spawn from snapshot: `Sandbox.create(snapshotId)`
- List, delete snapshots via SDK
- Requires envd v0.5.0+

**Use Cases**:
- Checkpointing agent progress
- Rollback points before risky operations
- Forking sandboxes for parallel exploration
- Skipping expensive setup (pre-warm with data loaded)
- State sharing across users/agents

---

## 7. Tool Use / MCP Support

**MCP Gateway**:
- Built-in MCP gateway runs inside sandboxes at `http://localhost:50005/mcp`
- 200+ pre-integrated MCP servers from Docker MCP Catalog
- HTTP endpoint with bearer token authentication

**Notable MCP Server Categories**:
- Databases: MongoDB, PostgreSQL, MySQL, SQLite, Redis, Neo4j, Elasticsearch, etc.
- Cloud: AWS, GCP, Azure, Kubernetes, Docker Hub
- Web: Firecrawl, Browserbase, Playwright, Puppeteer
- Search: DuckDuckGo, Brave, Exa, Tavily, Perplexity
- Business: HubSpot, Salesforce, Stripe, Notion, Airtable, Jira, Confluence
- Dev: GitHub, GitLab, CircleCI, Buildkite, JetBrains
- Security: Semgrep, SonarQube, StackHawk
- AI: Hugging Face
- Monitoring: Grafana, Prometheus, Dynatrace

**Configuration**: MCP servers configured via key-value pairs during sandbox creation (API keys, etc.)

**Custom MCP Servers**: Can run custom servers within sandboxes

**External Connection**: MCP URL and token available via `sandbox.getMcpUrl()` / `sandbox.getMcpToken()` for connecting from external clients (Claude, OpenAI Agents, etc.)

---

## 8. Multi-Agent Support

**Pre-built Agent Templates**:
- Claude Code (`claude` template)
- Codex (OpenAI)
- Amp
- OpenCode (open-source, multi-provider)

**Integration Pattern**:
1. Create sandbox from agent template
2. Agent gets full Linux environment: terminal, filesystem, git, package managers
3. Agent operates autonomously inside the sandbox
4. Extract results via SDK (git diffs, structured output, files)

**Parallel Execution**:
- Snapshots enable forking: create one sandbox, snapshot it, spawn multiple copies for parallel agent exploration
- Concurrent sandbox limits: 20 (Hobby), 100-1,100 (Pro), custom (Enterprise)
- Creation rate: 1/sec (Hobby), 5/sec (Pro), custom (Enterprise)

**Claude Code Specifics**:
- Headless mode with `-p` flag and `--dangerously-skip-permissions`
- Output formats: `--output-format json` or `--output-format stream-json`
- Session persistence via `--session-id`
- Custom CLAUDE.md injection
- MCP tool access

---

## 9. SDK / Client Libraries

**JavaScript/TypeScript SDK** (`e2b` npm package, v2.14.1+):
- Full async/await API
- Sandbox class with static and instance methods
- Sub-APIs: `commands`, `files`, `git`, `pty`
- Streaming via callbacks
- Paginated listing

**Python SDK** (`e2b` pip package, v2.14.1+):
- Both sync and async variants (`Sandbox` and `AsyncSandbox`)
- Equivalent feature parity with JS SDK
- Pythonic naming (`on_stdout`, `run_code`, `watch_dir`, etc.)

**CLI** (`e2b`):
- Auth, sandbox creation/connection/listing/shutdown
- Template building
- Command execution in sandboxes
- Sandbox metrics

**REST API**:
- Full REST API for sandbox management, templates, events, tags, teams
- OpenAPI spec available at `e2b.mintlify.app/openapi-public.yml`
- API key authentication via `X-API-Key` header

---

## 10. Code Execution Capabilities (Code Interpreter)

**Supported Languages**:
- Python (primary, with pre-installed data science libraries)
- JavaScript / TypeScript
- R
- Java
- Bash
- Any custom language via custom templates

**Execution Contexts**:
- Multiple independent execution contexts per sandbox
- Each context maintains its own isolated state (variables, imports, etc.)
- Configurable working directory and language per context
- Context lifecycle: create, list, restart (clears state), remove
- Default context available without explicit creation

**Streaming**:
- `onStdout`, `onStderr`, `onError` callbacks during code execution
- `onResult` callback for computed artifacts (charts, tables, structured data)
- Each event includes: `line`, `error` (boolean), `timestamp` (Unix microseconds)

**Data Visualization**:
- Static charts (matplotlib, seaborn, etc.)
- Interactive charts (plotly, bokeh, etc.)
- Results returned as artifacts from `runCode()`

---

## 11. Deployment Model

**E2B Cloud (default)**:
- Fully managed SaaS
- Infrastructure on GCP (primary), AWS (supported)
- No self-hosting required

**BYOC (Bring Your Own Cloud)** -- Enterprise only:
- Supported: AWS, GCP (Azure planned)
- Dual infrastructure: sandbox VMs run in customer VPC; orchestration from E2B Cloud
- Sensitive data (templates, sandbox traffic, logs) stays in customer VPC
- Only anonymized system metrics sent to E2B Cloud
- Components: Orchestrator, Edge Controller, Monitoring, Storage
- Provisioned via Terraform
- TLS encryption for E2B Cloud-to-BYOC communication
- VPC peering for private connectivity
- Autoscaling: limited (manual horizontal scaling currently; full autoscaling planned)

**Open-Source Infrastructure**:
- Infrastructure repo: github.com/e2b-dev/infra
- Written primarily in Go (84.6%)
- Uses Nomad for orchestration, Consul for service coordination, Terraform for IaC
- Self-hosting guide available

---

## 12. Image / Environment Customization (Templates)

**Template Definition** (programmatic API, replaces Dockerfiles):
- Fluent/chainable API in JS/Python
- Base images: Ubuntu, Debian, Python, Node.js, Bun, or any Docker Hub image
- `fromTemplate()` to extend existing team templates
- `e2bdev/base` as optimized default base image

**Configuration Methods**:
- `aptInstall()`, `pipInstall()`, `npmInstall()`, `bunInstall()` for packages
- `runCmd()` for arbitrary shell commands
- `copy()` / `copyItems()` for files with permission options
- `setEnvs()` for environment variables (build-time only)
- `setUser()` / `setWorkdir()` for user and working directory
- `setStartCmd()` with ready command (e.g., `waitForPort()`, `waitForProcess()`, `waitForFile()`, `waitForTimeout()`)
- `gitClone()` for repository cloning
- `makeDir()`, `makeSymlink()`, `remove()`, `rename()` for filesystem setup

**Build Process**:
1. Container created from definition
2. Filesystem extracted, dependencies installed
3. Start command executed (if specified)
4. Ready command polled in loop until exit code 0
5. Snapshot created (filesystem + running processes serialized)
6. Snapshot serves as template -- loads in ~80ms

**Build Features**:
- Sync and async build methods
- Layer-based caching (like Docker) -- team-level cache sharing
- Content-based file caching (survives layer invalidation)
- `skipCache()` for partial or full cache invalidation
- Custom CPU/memory for builds
- Build logging via callbacks
- Background build with status polling
- Private Docker registry support
- Tags for template versioning

**Docker Compatibility**:
- Dockerfile instructions mapped to template methods (RUN -> runCmd, etc.)
- Multi-stage Dockerfiles NOT supported
- EXPOSE/VOLUME directives ignored

---

## 13. Resource Management

**Default Resources**: 2 vCPU, 1 GB RAM (per sandbox)

**Customizable per template build**:
- CPU: specified in cores
- Memory: specified in MB
- Disk: 10 GB (Hobby), 20 GB (Pro)

**Monitoring**:
- Metrics sampled every 5 seconds
- CPU: usage percentage, core count
- Memory: bytes used, bytes total (typical ~507 MB total allocation observed)
- Disk: bytes used, bytes total (typical ~2.5 GB observed)
- Access via SDK instance/static methods or CLI

**Rate Limits**:
- API (lifecycle): 20,000 requests / 30 seconds
- API (operations): 40,000 requests / 60 seconds per IP
- Concurrent sandboxes: 20 (Hobby), 100-1,100 (Pro), 1,100+ (Enterprise)
- Creation rate: 1/sec (Hobby), 5/sec (Pro), 5+/sec (Enterprise)
- Network: 2,500 connections per sandbox
- HTTP 429 returned when exceeded

---

## 14. Security Model

**Isolation**:
- Hardware-level VM isolation via Firecracker + KVM (not containers)
- Each sandbox has its own Linux kernel
- Minimal VMM attack surface (50k lines of Rust)

**Authentication**:
- API key authentication for all API/SDK calls
- Secure access (X-Access-Token) for SDK-to-sandbox controller communication (enabled by default since SDK v2.0.0)
- Traffic access tokens for restricting public sandbox URL access
- MCP bearer token authentication

**Network Security**:
- Deny-all option (`allowInternetAccess: false`)
- Fine-grained allow/deny lists for outbound traffic (IPs, CIDRs, domains)
- Public traffic restriction with token-gated access

**Credential Handling**:
- Git credentials stored in sandbox are readable by any process in that sandbox (documented risk)
- Environment variables are not private within the OS (documented)

**Webhook Security**:
- HMAC-SHA256 signature verification

---

## 15. Streaming Capabilities

**Command Output Streaming**:
- `onStdout` / `onStderr` callbacks on `commands.run()`
- Real-time, as-it-happens delivery

**Code Interpreter Streaming**:
- `onStdout`, `onStderr`, `onError` for execution output
- `onResult` for computed artifacts (charts, data, visualizations)
- Events include line content, error flag, and microsecond-precision timestamps

**PTY Streaming**:
- Full bidirectional real-time streaming
- ANSI escape sequence support
- Persistent sessions with reconnection

**Desktop/Computer Use Streaming**:
- VNC-based real-time desktop streaming
- Screenshot capture API for agent loops

**Lifecycle Event Streaming**:
- Webhooks deliver real-time notifications for lifecycle state changes
- Pull-based event API with pagination as alternative

---

## Pricing Summary

| Plan | Monthly | Free Credits | Max Runtime | Concurrent | Creation Rate |
|------|---------|-------------|-------------|------------|---------------|
| Hobby | $0 | $100 | 1 hour | 20 | 1/sec |
| Pro | $150 | $100 | 24 hours | 100-1,100 | 5/sec |
| Enterprise | Custom | Custom | Custom | 1,100+ | Custom |

- Billed per second for compute while sandbox is running
- Default: 2 vCPU, 1 GB RAM
- Auto-pause stops billing while preserving state

---

## Key Differentiators vs. agentOS

| Aspect | E2B | agentOS |
|--------|-----|---------|
| Isolation | Firecracker microVMs (hardware KVM) | In-process JS kernel (WASM/V8/Pyodide) |
| Deployment | Cloud SaaS / BYOC | In-process library |
| Boot time | ~125ms (from snapshot: ~80ms) | Instant (same process) |
| Persistence | Pause/resume, snapshots, volumes | N/A (ephemeral) |
| Languages | Any (full Linux VM) | JS/TS, WASM, Python (Pyodide) |
| Native binaries | Full Linux ELF support | Not supported |
| Networking | Real Linux networking, public URLs | Kernel-managed virtual sockets |
| MCP | 200+ built-in MCP servers | N/A |
| Multi-agent | Snapshot forking, parallel sandboxes | Single VM per session |
| Cost model | Per-second cloud billing | Zero (local compute) |
| Security boundary | Hardware VM isolation | Process-level (V8 isolate + WASM) |
