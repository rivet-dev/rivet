import type { IncomingMessage, ServerResponse } from "node:http";
import fs from "node:fs";
import path from "node:path";
import { contentType } from "./mime";
import { wantsHtmlDocument } from "./probe";

export function createStaticAndProbeHandler(options: {
	publicDir: string;
	getServiceName: () => string;
}) {
	const { publicDir, getServiceName } = options;

	return async function handleStaticAndProbes(
		req: IncomingMessage,
		res: ServerResponse,
		url: URL,
	): Promise<void> {
		if (url.pathname === "/health") {
			res.writeHead(200, { "Content-Type": "application/json; charset=utf-8" });
			res.end(JSON.stringify({ status: "ok" }));
			return;
		}

		if (
			url.pathname === "/" &&
			req.method === "GET" &&
			!wantsHtmlDocument(req)
		) {
			res.writeHead(200, { "Content-Type": "application/json; charset=utf-8" });
			res.end(JSON.stringify({ status: "ok", service: getServiceName() }));
			return;
		}

		const pathname = url.pathname === "/" ? "/index.html" : url.pathname;
		const safe = path.normalize(pathname).replace(/^(\.\.(\/|\\|$))+/, "");
		const filePath = path.join(publicDir, safe);

		if (!filePath.startsWith(publicDir)) {
			res.writeHead(403);
			res.end();
			return;
		}

		await new Promise<void>((resolve) => {
			fs.readFile(filePath, (err, data) => {
				if (err) {
					if (pathname !== "/index.html") {
						const fallback = path.join(publicDir, "index.html");
						fs.readFile(fallback, (err2, html) => {
							if (err2) {
								res.writeHead(404);
								res.end();
								resolve();
								return;
							}
							res.writeHead(200, { "Content-Type": "text/html; charset=utf-8" });
							res.end(html);
							resolve();
						});
						return;
					}
					res.writeHead(404);
					res.end();
					resolve();
					return;
				}
				res.writeHead(200, { "Content-Type": contentType(filePath) });
				res.end(data);
				resolve();
			});
		});
	};
}
