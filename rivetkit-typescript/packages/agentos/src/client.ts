/**
 * `@rivet-dev/agentos/client` — Agent OS client surface.
 *
 * Re-exports RivetKit's client entry (`createClient`, `ActorHandle`,
 * `ActorConn`, etc.). This is deliberately kept OFF the root module so that
 * server/Node actor code that imports `@rivet-dev/agentos` never transitively
 * pulls in the browser/client bundle.
 *
 * The browser export condition is preserved through to `rivetkit/client`
 * (which is external here), so consumers resolving this subpath in a browser
 * environment still get RivetKit's browser client build.
 */

export * from "rivetkit/client";
