# Terminology

- Client - the user/app connecting to Rivet
- Runner - the client-side runner code
- Runner Protocol - Rivet <-> Runner communication protocol defined as BARE (see engine/sdks/schemas/runner-protocol)
- Runner WF - the rivet-side runner, manages runner lifecycle
- Actor WF - rivet-side actor
- Gateway - User requests connect to this to communicate with actors running on runners
- Runner WS - The runner connects to this to communicate to Rivet
