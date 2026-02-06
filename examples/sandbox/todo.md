# Sandbox Migration Progress

## Phase 1: Copy all actors locally
- [x] counter/ group (counter, counter-conn, conn-params, lifecycle)
- [x] actions/ group (action-inputs, action-types, action-timeout, error-handling)
- [x] state/ group (actor-onstatechange, metadata, vars, kv, large-payloads)
- [x] connections/ group (conn-state, reject-connection, request-access)
- [x] http/ group (raw-http, raw-http-request-properties, raw-websocket, raw-fetch-counter, raw-websocket-chat-room)
- [x] lifecycle/ group (run, sleep, scheduled, destroy, hibernation)
- [x] queue/ additions (queue.ts from fixtures)
- [x] workflow/ additions (workflow-fixtures.ts from fixtures with import transforms)
- [x] inter-actor/ group (inventory, checkout from cross-actor-actions)
- [x] testing/ group (inline-client)
- [x] ai/ group (ai-agent)

## Phase 2: Update imports
- [x] Rewrite actors.ts to use only local imports
- [x] Remove ugcCounter from actors.ts
- [x] Remove ugcCounter from page-data.ts
- [x] Delete src/actors/ugc-counter.ts
- [x] Update vite.config.ts (remove @/ alias)
- [x] Copy ai-agent supporting files (my-tools.ts, types.ts)

## Phase 3: Verify dev server starts
- [x] Run pnpm dev and verify no errors (72 actors loaded)
- [x] Fix any import/build issues

## Phase 4: Test every screen with agent-browser
- [x] Overview > Welcome
- [x] Overview > Registry and Keys
- [x] Core API > Actor Configuration
- [x] Core API > Actions
- [x] Core API > Types and Helper Types
- [x] Core API > Action Timeouts
- [x] Core API > Errors
- [x] Core API > Appearance
- [x] State and Storage > State Basics
- [x] State and Storage > On State Change
- [x] State and Storage > Sharing and Joining State
- [x] State and Storage > Metadata
- [x] State and Storage > Ephemeral Variables
- [x] State and Storage > KV Storage
- [x] State and Storage > External SQL
- [x] State and Storage > Large Payloads
- [x] Realtime and Connections > Connections and Presence
- [x] Realtime and Connections > Events and Broadcasts
- [x] Realtime and Connections > Direct Connection Messaging
- [x] Realtime and Connections > Connection Gating
- [x] Realtime and Connections > Request Object Access
- [x] HTTP and WebSocket > HTTP API
- [x] HTTP and WebSocket > Request Handler
- [x] HTTP and WebSocket > WebSocket Handler
- [x] HTTP and WebSocket > Fetch and WebSocket Handler
- [x] HTTP and WebSocket > Raw HTTP
- [x] HTTP and WebSocket > Raw WebSocket
- [x] Lifecycle and Scheduling > Lifecycle Hooks
- [x] Lifecycle and Scheduling > Run Handler
- [x] Lifecycle and Scheduling > Sleep and Wake
- [x] Lifecycle and Scheduling > Schedule
- [x] Lifecycle and Scheduling > Destroy
- [x] Lifecycle and Scheduling > Hibernation
- [x] Lifecycle and Scheduling > Versions
- [x] Lifecycle and Scheduling > Scaling
- [x] Queues > Queue Basics
- [x] Queues > Queue Patterns
- [x] Queues > Queue in Run Loop
- [x] Workflows > Workflow Overview
- [x] Workflows > Steps
- [x] Workflows > Sleep
- [x] Workflows > Loops
- [x] Workflows > Listen
- [x] Workflows > Join
- [x] Workflows > Race
- [x] Workflows > Rollback
- [x] Inter-Actor and Patterns > Communicating Between Actors
- [x] Inter-Actor and Patterns > Design Patterns
- [x] Testing and Debugging > Testing
- [x] AI > AI and User-Generated Actors

## Bugs Fixed During Testing
- [x] React crash: `setLog is not defined` in ActionPanel (referenced vars from RawWebSocketPanel)
- [x] RivetKit crash: ActorInspector class field initializer ran before constructor set #config
- [x] Server crash: `inventory` and `checkout` actors required createState input but sandbox creates without it
- [x] Raw WebSocket: `actor.handle.webSocket()` returns a Promise, needed `await`
- [x] Made all workflow actor createState inputs optional with defaults
