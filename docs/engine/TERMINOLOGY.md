# Terminology

- Rivet Engine - The binary running everything related to Rivet
- Client - the user/app connecting to Rivet
- Runner - the client-side runner code
- Runner Protocol - Rivet <-> Runner communication protocol defined as BARE (see engine/sdks/schemas/runner-protocol)
- Runner WF - the rivet-side runner, manages runner lifecycle
- Actor WF - rivet-side actor
- Gateway - A portion of Guard responsible for proxying requests and websockets to actors running on runners
- Runner WS - The runner connects to this to communicate to Rivet
