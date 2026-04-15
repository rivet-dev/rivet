// The engine guard (public HTTP API) runs on this internal port.
// The rivetkit manager runs on port 6420 (managerPort default) and proxies to
// the engine here, so clients always connect to the manager on 6420.
export const ENGINE_PORT = 6423;
export const ENGINE_ENDPOINT = `http://127.0.0.1:${ENGINE_PORT}`;
