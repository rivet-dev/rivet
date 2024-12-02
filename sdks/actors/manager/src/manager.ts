import { Context as HonoContext, Hono } from "hono";
import { cors } from "hono/cors";
import { RivetClient, RivetClientClient } from "@rivet-gg/api";
import { queryActor } from "./query_exec.ts";
import { assertExists } from "@std/assert/exists";
import {
	assertUnreachable,
	PORT_NAME,
	RivetEnvironment,
} from "../../common/src/utils.ts";
import {
	ActorsRequest,
	ActorsResponse,
} from "../../manager-protocol/src/mod.ts";

export class Manager {
	private readonly rivetClient: RivetClientClient;
	private readonly environment: RivetEnvironment;

	constructor() {
		//const endpoint = Deno.env.get("RIVET_API_ENDPOINT");
		//assertExists(endpoint, "missing RIVET_API_ENDPOINT");
		//const token = Deno.env.get("RIVET_SERVICE_TOKEN");
		//assertExists(token, "missing RIVET_SERVICE_TOKEN");
		//const project = Deno.env.get("RIVET_PROJECT");
		//const environment = Deno.env.get("RIVET_ENVIRONMENT");

		this.rivetClient = new RivetClientClient({
			environment: endpoint,
			token,
		});

		this.environment = {
			project,
			environment,
		};
	}

	run() {
		const portStr = Deno.env.get("PORT_http") ?? Deno.env.get("HTTP_PORT");
		if (!portStr) throw "Missing port";
		const port = parseInt(portStr);
		if (!isFinite(port)) throw "Invalid port";

		const app = new Hono();

		app.use("/*", cors());

		app.post("/actors", async (c: HonoContext) => {
			// Get actor
			const body: ActorsRequest = await c.req.json();
			const actor = await queryActor(rivetClient, environment, body.query);

			// Fetch port
			const httpPort = actor.network.ports[PORT_NAME];
			assertExists(httpPort, "missing port");
			const hostname = httpPort.publicHostname;
			assertExists(hostname);
			const port = httpPort.publicPort;
			assertExists(port);

			let isTls: boolean = false;
			switch (httpPort.protocol) {
				case "https":
					isTls = true;
					break;
				case "http":
				case "tcp":
					isTls = false;
					break;
				case "tcp_tls":
				case "udp":
					throw new Error(`Invalid protocol ${httpPort.protocol}`);
				default:
					assertUnreachable(httpPort.protocol);
			}

			const endpoint = `${isTls ? "https" : "http"}://${hostname}:${port}`;

			return c.json({ endpoint } satisfies ActorsResponse);
		});

		Deno.serve({
			port,
			hostname: "0.0.0.0",
		}, app.fetch);
	}
}
