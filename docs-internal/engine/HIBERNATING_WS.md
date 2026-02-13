# Hibernating Websockets

## Lifecycle

1. Client establishes a websocket connection to an actor via Rivet, which is managed by Guard (see GUARD.md)
2. Guard checks to see if the actor is awake. If it is, skip step 3
3. If the actor is not awake, send a Wake signal to its workflow. This will make the actor allocate to an existing runner, or in the case of serverless, start a new runner and allocate to that
4. Guard sends the runner a ToClientWebSocketOpen message a via the runner protocol
5. The runner sends back ToServerWebSocketOpen to acknowledge the connection
	- The runner must set `.canHibernate = true` for hibernation to work
6. At this point the websocket connection is fully established and any websocket messages sent by the client are proxied through Guard to the runner to be delegated to the actor
7. Should the actor go to sleep, the runner will close the websocket by sending ToServerWebSocketClose with `.hibernate = true` via the runner protocol
8. Guard receives that the websocket has closed on the runner side and starts hibernating. During hibernation nothing happens.
9. 
	- If the actor is awoken from any other source, go to step 6. We do not send a ToClientWebSocketOpen message in this case
	- If the client sends a websocket message during websocket hibernation, go to step 2
	- If the client closes the websocket, the actor is rewoken (if not already running) and sent a ToClientWebSocketClose

## State

To facilitate state management on the runner side (specifically via RivetKit), each hibernating websocket runs a keepalive loop which periodically stores a value to UDB marking it as active.

When a client websocket closes during hibernation, this value is cleared.

When a runner receives a CommandStartActor message via the runner protocol, it contains information about which hibernating requests are still active.
