export const ENGINE_PORT = 6420;
export const ENGINE_ENDPOINT = `http://127.0.0.1:${ENGINE_PORT}`;

export function isLocalEngineEndpoint(endpoint: string): boolean {
	let url: URL;
	try {
		url = new URL(endpoint);
	} catch {
		return false;
	}

	const hostname = url.hostname.toLowerCase();
	return (
		hostname === "localhost" ||
		hostname === "0.0.0.0" ||
		hostname === "::" ||
		hostname === "[::]" ||
		hostname === "::1" ||
		hostname === "[::1]" ||
		/^127(?:\.\d{1,3}){0,3}$/.test(hostname)
	);
}
