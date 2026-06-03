# agentOS Documentation Spec

## Constraints

- **Code-heavy**: Every feature page must lead with working code examples, not prose. Follow the style of the existing actor docs.
- **Style reference docs**: Use these as structural templates:
  - `website/src/content/docs/actors/queues.mdx` - best example of pattern-per-section layout
  - `website/src/content/docs/actors/sqlite.mdx` - best example of raw API + ORM dual approach
  - `website/src/content/docs/actors/workflows.mdx` - best example of step/loop primitives
- **All examples use `agentOs()` actor**, never raw `AgentOs.create()`. All show both server and client.
- **`<CodeGroup workspace>`** with `registry.ts` + `client.ts` for every pattern.
- **Page structure**: Brief intro bullets (3-5), code-heavy body (one pattern per H2), recommendations/pitfalls at bottom.
- **Orchestration pages** show combining `agentOs()` with standard actor primitives (`workflow()`, `queue()`, SQLite, events).
- **All code blocks must typecheck** per `website/CLAUDE.md` rules.
- **Sitemap**: Add all pages under the existing agentOS section in `website/src/sitemap/mod.ts`.
- **Frontmatter**: Every `.mdx` file requires `title`, `description`, `skill: true`.

## Sitemap Structure

All pages live under `/docs/agent-os/` in `website/src/sitemap/mod.ts`:

```
agentOS
├── General
│   ├── Overview                    /docs/agent-os
│   └── Quickstart                  /docs/agent-os/quickstart
├── Agent
│   ├── Sessions                    /docs/agent-os/sessions
│   ├── Permissions                 /docs/agent-os/permissions
│   ├── Tools                       /docs/agent-os/tools
│   ├── Supported Agents            /docs/agent-os/supported-agents
│   ├── LLM Gateway (TODO)         /docs/agent-os/llm-gateway
│   └── Connecting to Private APIs  /docs/agent-os/private-apis
├── Operating System
│   ├── Filesystem                  /docs/agent-os/filesystem
│   ├── Processes & Shell           /docs/agent-os/processes
│   ├── Networking & Previews       /docs/agent-os/networking
│   ├── Cron Jobs                   /docs/agent-os/cron
│   └── Sandbox Mounting           /docs/agent-os/sandbox
├── Orchestration
│   ├── Multiplayer                 /docs/agent-os/multiplayer
│   ├── Workflow Automation         /docs/agent-os/workflows
│   ├── Queues                      /docs/agent-os/queues
│   └── SQLite Memory               /docs/agent-os/memory
└── Reference
    ├── Architecture                /docs/agent-os/architecture
    ├── Deployment                  /docs/agent-os/deployment
    ├── Security                    /docs/agent-os/security
    ├── Configuration               /docs/agent-os/configuration
    ├── Persistence & Sleep         /docs/agent-os/persistence
    ├── Events                      /docs/agent-os/events
    └── Performance                 /docs/agent-os/performance
```

## Page Specs

### General

#### Overview (`/docs/agent-os`)
Refresh existing page. What agentOS is, why it exists, high-level feature map pointing to each section.

#### Quickstart (`/docs/agent-os/quickstart`)
Rewrite existing page. Registry setup with `agentOs()` actor, client creation, create session, send prompt, stream response, read a file. Two-file CodeGroup.

---

### Agent

#### Sessions (`/docs/agent-os/sessions`)
- `createSession` with agent type + options
- `sendPrompt` / `cancelPrompt`
- `resumeSession` / `closeSession` / `destroySession`
- Subscribing to `sessionEvent` for streaming responses
- `setMode` / `setModel` / `setThoughtLevel` runtime config
- `getEvents` / `getSequencedEvents` for replay
- `listPersistedSessions` / `getSessionEvents` for history
- `rawSend` for custom JSON-RPC
- Universal transcript format (ACP), automatic transcript persistence

#### Permissions (`/docs/agent-os/permissions`)
- `respondPermission` for tool-use approval
- Subscribing to `permissionRequest` events
- `onPermissionRequest` server-side hook
- Auto-approve vs human-in-the-loop patterns
- Example: agent requests file write permission, client approves/denies

#### Tools (`/docs/agent-os/tools`)
- Exposing custom tools to agents via simple API
- Tool execution lifecycle (request, execution, result)
- Built-in vs custom tool patterns
- Example: define a custom tool, agent calls it, handle result

#### Supported Agents (`/docs/agent-os/supported-agents`)
- Claude Code, Codex, OpenCode, PI
- Setup guide for each agent type
- Agent-specific configuration and capabilities

#### LLM Gateway (`/docs/agent-os/llm-gateway`)
- TODO
- LLM metering

#### Connecting to Private APIs (`/docs/agent-os/private-apis`)
- Exposing internal services to agents securely
- Auth forwarding patterns (tokens, headers)
- Isolated private network model
- Example: agent calls an internal REST API through the host

---

### Operating System

#### Filesystem (`/docs/agent-os/filesystem`)
- `writeFile` / `readFile` round-trip
- `mkdir` / `readdir` / `readdirRecursive`
- `stat` / `exists` / `move` / `deleteFile`
- Batch: `writeFiles` / `readFiles`
- `mountFs` / `unmountFs` - mount anything as a filesystem
- `listAgents` for agent registry entries
- Note: `/home/user` backed by SQLite VFS automatically
- Example: write files, read directory tree, mount external storage

#### Processes & Shell (`/docs/agent-os/processes`)
- One-shot: `exec` (stdout, stderr, exitCode)
- Long-running: `spawn` + `processOutput` / `processExit` events
- Stdin: `writeProcessStdin` / `closeProcessStdin`
- Lifecycle: `waitProcess`, `listProcesses`, `getProcess`, `stopProcess`, `killProcess`
- System-wide: `allProcesses` / `processTree`
- Interactive shell: `openShell` / `writeShell` / `resizeShell` / `closeShell` + `shellData` events
- Example: spawn dev server, stream logs; open shell, pipe terminal I/O

#### Networking & Previews (`/docs/agent-os/networking`)
- `vmFetch` to proxy HTTP into VM services (method, headers, body)
- `createSignedPreviewUrl` for time-limited public URLs to VM services
- `expireSignedPreviewUrl` to revoke
- Token expiration defaults/limits, CORS
- Example: spawn HTTP server in VM, fetch from it; create shareable preview URL

#### Cron Jobs (`/docs/agent-os/cron`)
- `scheduleCron` with cron expression + action (`exec` or `session`)
- `listCronJobs` / `cancelCronJob`
- Subscribing to `cronEvent` events
- Overlap modes: `allow`, `skip`, `queue`
- Example: schedule periodic cleanup command, schedule recurring agent session

#### Sandbox Mounting (`/docs/agent-os/sandbox`)
- Hybrid OS model: when and why to extend with a full sandbox
- Escalating from lightweight VM to sandbox for untrusted workloads
- Configuration and setup
- Example: configure sandbox mounting for an agent-os actor

---

### Orchestration

These pages demonstrate combining the `agentOs()` actor with standard Rivet Actor primitives.

#### Multiplayer (`/docs/agent-os/multiplayer`)
- Multiple clients connected to same agent-os actor
- Broadcasting events to all subscribers (session events, process output, shell data)
- Collaborative patterns: one user prompts, others observe
- Handoff between human and agent
- Example: two clients subscribe to same session, both see streaming output

#### Workflow Automation (`/docs/agent-os/workflows`)
- Using actor `workflow()` to orchestrate multi-step agent tasks
- Steps that create sessions, send prompts, wait for results
- Error handling and retry with `ctx.step()`
- Chaining agents: output of one session feeds into next
- Example: workflow that clones repo, runs agent to fix bug, runs tests, reports result

#### Queues (`/docs/agent-os/queues`)
- Using actor queues to serialize agent work (queue commands)
- Ingesting tasks from external systems into agent queue
- Completable messages for request/response with agents
- Backpressure and rate limiting agent sessions
- Example: queue of code review requests, agent processes one at a time

#### SQLite Memory (`/docs/agent-os/memory`)
- Using actor SQLite as long-term agent memory
- Storing conversation summaries, tool results, learned context
- Querying memory across sessions
- Schema migrations for memory evolution
- Example: agent stores findings in SQLite, retrieves context in future sessions

---

### Reference

#### Architecture (`/docs/agent-os/architecture`)
- System diagram: client -> actor -> VM (secure-exec kernel)
- Powered by WebAssembly and V8 isolates (same as Cloudflare & Chromium)
- Core vs actor layer separation
- SQLite VFS layer for filesystem persistence
- Event flow: VM -> actor -> client
- Session persistence model (tables, replay)

#### Deployment (`/docs/agent-os/deployment`)
- Rivet Cloud or your own infrastructure
- Easy on-prem deployment
- Runtime requirements (secure-exec kernel, VM support)
- Driver configuration for different environments
- Scaling considerations
- Resource limits and tuning

#### Security (`/docs/agent-os/security`)
- Restrict CPU & memory granularly
- Programmatic network control
- Custom authentication (`onBeforeConnect`)
- Isolated private network
- VM isolation model (WebAssembly + V8 isolates, same as Cloudflare & Chromium)
- Preview URL token security (generation, expiration, revocation)
- Permission system for tool use
- Filesystem isolation: mount boundaries

#### Configuration (`/docs/agent-os/configuration`)
- `agentOs({...})` factory options
- `options` (AgentOsOptions for VM creation, mounts)
- `preview` config (defaultExpiresInSeconds, maxExpiresInSeconds)
- `onBeforeConnect` / `onSessionEvent` / `onPermissionRequest` hooks
- Action timeout, sleep grace period

#### Persistence & Sleep (`/docs/agent-os/persistence`)
- SQLite VFS backing `/home/user`
- What prevents sleep: active sessions, processes, shells, hooks
- Sleep grace period (15 min default)
- What persists across sleep: filesystem, session records, events, preview tokens
- Sleep vs destroy behavior
- Persisted tables schema overview

#### Events (`/docs/agent-os/events`)
- Full event catalog with payload shapes:
  - `sessionEvent`, `permissionRequest`, `vmBooted`, `vmShutdown`
  - `processOutput`, `processExit`, `shellData`, `cronEvent`
- Client subscription patterns
- Event replay via `getEvents` / `getSequencedEvents`

#### Performance (`/docs/agent-os/performance`)
- Low overhead claims and data
- Benchmark data (same benchmarks as secure-exec)
- Comparison points

## Coverage Checklist (from landing page)

All items from the landing page are mapped to docs pages:

- [x] Claude Code, Codex, OpenCode, PI support → Supported Agents
- [x] Tool integration → Tools
- [x] Mount anything as a filesystem → Filesystem
- [x] Low overhead → Performance
- [x] Granular security → Security
- [x] Extend with sandbox (hybrid OS) → Sandbox Mounting
- [x] Runs on your infra → Deployment
- [x] Expose tools with simple API → Tools
- [x] Cron jobs → Cron Jobs
- [x] Preview URLs → Networking & Previews
- [x] Easy process observability → Processes & Shell
- [x] File system API → Filesystem
- [x] Simple sessions API → Sessions
- [x] LLM metering → LLM Gateway (TODO)
- [x] Universal transcript format (ACP) → Sessions
- [x] Automatic transcript persistence → Sessions + Persistence & Sleep
- [x] Multiplayer → Multiplayer
- [x] Automate with workflows → Workflow Automation
- [x] Queue commands → Queues
- [x] Restrict CPU & memory → Security
- [x] Programmatic network control → Security
- [x] Custom authentication → Security
- [x] Isolated private network → Security
- [x] Powered by WebAssembly and V8 isolates → Architecture + Security
- [x] Easy on-prem deploy → Deployment
- [x] Benchmarks → Performance
