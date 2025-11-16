# Hibernatable Connections

## Lifecycle

### New Connection

```mermaid
sequenceDiagram
	participant P as Pegboard
	participant R as Runner
	participant A as ActorDriver
	participant I as Instance
	participant D as ActorDefinition

	P->>R: ToClientWebSocketOpen
	R->>A: Runner.config.websocket
	A->>A: handleWebSocketConnection
	A->>I: ConnectionManager.prepareConn
	A->>D: ActorDefinition.onBeforeConnect
	A->>I: ActorDefinition.createConnState
	R->>R: WebSocketAdapter._handleOpen
	R->>A: open event
	A->>A: ConnectionManager.connectConn
	A->>I: ActorDefinition.onConnect
	note over A: TODO: persist
	R->>P: ToServerWebSocketOpen
```

### Restore Connection


```mermaid
sequenceDiagram
	participant P as Pegboard
	participant R as Runner
	participant A as ActorDriver
	participant I as Instance

	note over P,I: Actor start
	P->>R: ToClientCommands (CommandStartActor)
	R->>A: Runner.config.restoreHibernatingRequests
	note over R,A: TODO: This may be problematic
	R->>P: ToServerEvents (ActorStateRunning)

	note over P,I: Actor Start
	R->>A: Runner.config.onActorStart
	A->>I: Instance.#restoreExistingActor
	A->>A: ConnectionManager.restoreConnections
	note over A: Restores connections in to memory

	note over P,I: Conn Restoration
	R->>R: Tunnel.restoreHibernatingRequests
	note over R: Returns existing connections from actor state
	R->>A: Runner.config.websocket
	A->>A: handleWebSocketConnection
	A->>A: ConnectionManager.prepareConn
	A->>A: ConnectionManager.#reconnectHibernatableConn
```

TODO: Disconnecting stale conns
TODO: Disconnecting zombie conns

### Persisting Message Index

```mermaid
sequenceDiagram
	participant P as Pegboard
	participant R as Runner
	participant A as ActorDriver

	R->>R: _handleMessage
	R->>A: message event
	A->>A: update storage
	A->>A: saveAfter(TODO)
	note over A: ...after persist...
	A->>A: persist
	A->>A: afterPersist
	A->>R: TODO: ack callback
```

### Close Connection

```
TODO
```

