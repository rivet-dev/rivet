// Minimal ambient declarations for the Deno globals used by `serve`. Supabase
// Edge Functions run on Deno, but this package is typechecked in a Node-typed
// project, so we declare only what we use rather than depending on Deno types.
declare const Deno: {
	readFile(path: string | URL): Promise<Uint8Array>;
	env: { get(key: string): string | undefined };
	serve(handler: (request: Request) => Response | Promise<Response>): unknown;
};
