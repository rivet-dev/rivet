/**
 * `@rivet-dev/agentos/react` — Agent OS React bindings.
 *
 * Re-exports `@rivetkit/react` (`createRivetKit`, `createRivetKitWithClient`,
 * and the hooks). Kept on its own subpath so neither React nor the client
 * bundle is pulled into server code that imports the root module.
 */

export * from "@rivetkit/react";
