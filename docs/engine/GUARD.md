# Rivet Guard

Guard facilitates HTTP communication between the public internet and various internal rivet services, as well as tunnelling connections to actors.

## Routing

Guard uses request path and/or headers to route requests.

### Actors

Guard routes requests to actors when:
- the path matches `/gateway/{actor_id}/{...path}`
- the path matches `/gateway/{actor_id}@{token}/{...path}`
- when connecting a websocket, `Sec-Websocket-Protocol` consists of comma delimited dot separated pairs like `rivet_target.actor,rivet_actor.{actor_id}`
- otherwise, when the `X-Rivet-Target` header is set to `actor` and `X-Rivet-Actor` header is set to the actor id

### Runners

Guard accepts runner websocket connections when:
- the path matches `/runners/connect`
- `Sec-Websocket-Protocol` consists of comma delimited dot separated pairs like `rivet_target.runner`

### API Requests

Guard routes requests to the API layer when the `X-Rivet-Target` header is set to `api-public` or is unset.

## Proxying (Gateway)

The Gateway (a portion of Guard) acts as a proxy for requests and websockets to actors:

- Internally, the websocket connects to a websocket listener running on the Rivet Engine
- Rivet Engine transmits HTTP requests and websocket messages via the runner protocol to the actor's corresponding runner's websocket
	- The runner has a single websocket connection open to Guard which is independent from any client websocket connection
	- This single connection multiplexes all actor requests and websocket connections
- The runner delegates requests and websockets to actors
- The runner sends HTTP responses and websocket messages back to Rivet through is websocket via the runner protocol
- Rivet transforms the runner protocol messages into HTTP responses and websocket messages

### Websocket Hibernation

The Gateway allows us to implement hibernatable websockets (see HIBERNATING_WS.md) for actors. We can keep a client's websocket connection open while simultaneously allowing for actors to sleep, resulting in 0 usage when there is no traffic over the websocket. The actor is automatically awoken when a websocket message is transmitted to the Gateway.
