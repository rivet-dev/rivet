import type { IncomingMessage } from "node:http";

/** Build a WHATWG `Request` from an incoming Node request (for `registry.handler`). */
export function incomingMessageToRequest(
	req: IncomingMessage,
	port: number,
): Request {
	const xfProto = req.headers["x-forwarded-proto"];
	const proto =
		typeof xfProto === "string" ? xfProto.split(",")[0]?.trim() : undefined;
	const scheme = proto === "https" || proto === "http" ? proto : "http";
	const host = req.headers.host ?? `127.0.0.1:${port}`;
	return new Request(`${scheme}://${host}${req.url}`, {
		method: req.method,
		headers: req.headers as HeadersInit,
	});
}
