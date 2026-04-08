# Native Bridge Bugs

## Status
- **Actions**: PASS (increment, getCount, state persistence all work)
- **WebSocket**: FAIL (client-side connect timeout)
- **SQLite**: FAIL (sqlite3_open_v2 code 14 - CANTOPEN)

## Test command (actions - works)
```bash
cd rivetkit-typescript/packages/rivetkit
npx tsx tests/standalone-native-test.mts
```
Requires engine + test-envoy running, default namespace with metadata refreshed.

## WebSocket Bug

### Symptom
Client SDK `handle.connect()` times out. Server-side works fully: envoy receives `ToEnvoyWebSocketOpen`, wrapper fires `config.websocket()`, `EngineActorDriver.#envoyWebSocket` attaches listeners, open event dispatches, actor sends 128-byte message back. Envoy sends `ToRivetWebSocketOpen` AND `ToRivetWebSocketMessage`. But client-side WS never opens.

### Root cause
The engine's guard/gateway receives `ToRivetWebSocketOpen` from the envoy but does NOT complete the client-side WS upgrade. This is likely a guard bug with v2 actors - the guard's WS proxy code path may not handle the v2 tunnel response correctly.

### Evidence
- Envoy sends `ToRivetWebSocketOpen{canHibernate: false}` at timestamp X ✓
- Envoy sends `ToRivetWebSocketMessage{128 bytes}` immediately after ✓
- Engine log: `websocket failed: Connection reset without closing handshake` for the gateway WS
- Client-side WS closes without ever receiving the open event

### NOT a rivetkit-native issue
The server-side flow (TSFN, EventTarget ws, ws.send via WebSocketSender, actor framework) all work correctly. The bug is in how the engine's guard handles v2 actor WS connections.

### Additional issue: message_index conflict
The outgoing task in `actor.rs` (line ~459) sends `ToRivetWebSocketMessage` with hardcoded `message_index: 0`. But `send_actor_message` also sends messages starting at index 0. The guard may see duplicate indices and drop messages. Need to coordinate the message index between both paths.

### Reproduce
```bash
cd rivetkit-typescript/packages/rivetkit
npx tsx tests/standalone-native-test.mts
```
Actions pass (3/3), WebSocket fails with connect timeout. Check Rust logs with `RIVET_LOG_LEVEL=debug`.

### Code locations
- `engine/packages/guard-core/src/proxy_service.rs` line 1548-1554 - CustomServe WS handler
- `engine/packages/guard-core/src/proxy_service.rs` line 927 - handle_websocket_upgrade  
- `engine/sdks/rust/envoy-client/src/actor.rs` line ~459 - outgoing task with hardcoded message_index
- The guard's CustomServe handler (from the routing fn) should proxy ToRivetWebSocketOpen back to the client but doesn't complete the upgrade

## SQLite Bug

### Symptom
`sqlite3_open_v2 failed with code 14` (SQLITE_CANTOPEN)

### Root cause
The native SQLite VFS (`rivetkit-native/src/database.rs`) creates an `EnvoyKv` adapter that routes KV operations through the `EnvoyHandle`. But the VFS registration or database open may fail because:
1. The actor isn't ready when the DB tries to open
2. The VFS name conflicts
3. The KV batch_get returns unexpected data format

### What to investigate
- Add logging to `EnvoyKv` trait methods in `rivetkit-native/src/database.rs`
- Check if `open_database_from_envoy` is called at the right time
- Verify the envoy handle's KV methods work for the actor

### Code locations
- `rivetkit-native/src/database.rs` - EnvoyKv impl + open_database_from_envoy
- `rivetkit-typescript/packages/sqlite-native/src/vfs.rs` - KvVfs::register + open_database
- `src/drivers/engine/actor-driver.ts` line ~570 - getNativeSqliteProvider
