import http from "node:http";
import { serviceName } from "../config/service-name";
import { incomingMessageToRequest } from "./web-request";
import { pipeWebResponseToNode } from "./pipe-response";
import { createStaticAndProbeHandler } from "./serve-static";

export function startProductionServer(options: {
	registry: { handler: (req: Request) => Promise<Response> };
	port: number;
	publicDir: string;
}): void {
	const { registry, port, publicDir } = options;
	const handleStaticAndProbes = createStaticAndProbeHandler({
		publicDir,
		getServiceName: serviceName,
	});

	const server = http.createServer((req, res) => {
		void (async () => {
			const url = new URL(req.url ?? "/", `http://127.0.0.1:${port}`);

			if (url.pathname === "/api/rivet" || url.pathname.startsWith("/api/rivet/")) {
				try {
					const webReq = incomingMessageToRequest(req, port);
					const webRes = await registry.handler(webReq);
					await pipeWebResponseToNode(res, webRes);
				} catch (err) {
					console.error(err);
					if (!res.headersSent) {
						res.writeHead(500, {
							"Content-Type": "application/json; charset=utf-8",
						});
						res.end(JSON.stringify({ error: "rivet_handler_failed" }));
					}
				}
				return;
			}

			await handleStaticAndProbes(req, res, url);
		})().catch((err) => {
			console.error(err);
			if (!res.headersSent) {
				res.writeHead(500);
				res.end();
			}
		});
	});

	server.listen(port, "0.0.0.0", () => {
		console.log(
			`${serviceName()} — static + /health on http://0.0.0.0:${port} (Rivet Cloud serverless; RIVET_ENDPOINT set)`,
		);
	});
}
