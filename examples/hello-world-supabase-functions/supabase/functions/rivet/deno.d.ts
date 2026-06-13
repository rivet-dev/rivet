declare const Deno: {
	readFile(path: string | URL): Promise<Uint8Array>;
	env: { get(key: string): string | undefined };
	serve(handler: (request: Request) => Response | Promise<Response>): void;
};
