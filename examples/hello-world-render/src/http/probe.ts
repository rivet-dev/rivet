import type { IncomingMessage } from "node:http";

/**
 * Serve SPA HTML only for real browser navigations. Health checks hit `GET /` and expect JSON;
 * many probes send `Accept: text/html` without `Sec-Fetch-*` headers.
 */
export function wantsHtmlDocument(req: IncomingMessage): boolean {
	if (req.headers["sec-fetch-dest"] === "document") return true;
	if (req.headers["sec-fetch-mode"] === "navigate") return true;
	return false;
}
