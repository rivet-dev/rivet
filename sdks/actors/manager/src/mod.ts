import { Context as HonoContext, Hono } from "hono";
import { cors } from "hono/cors";
import { ActorContext } from "@rivet-gg/actors-core";
import { RivetClient } from "@rivet-gg/api";
import { queryActor } from "./query_exec.ts";
import { assertExists } from "@std/assert/exists";
import { assertUnreachable, RivetEnvironment } from "../../common/src/utils.ts";
import { PORT_NAME } from "../../common/src/network.ts";
import {
	ActorsRequest,
	ActorsResponse,
	RivetConfigResponse,
} from "../../manager-protocol/src/mod.ts";

interface AccessConfig {
	builds: Build[];
}

// all actors with public access:
interface Build {
	tags: any;
}

export default class Manager {
	private readonly endpoint: string;
	private readonly rivetClient: RivetClient;
	private readonly environment: RivetEnvironment;

	constructor(private readonly ctx: ActorContext) {
		const endpoint = Deno.env.get("RIVET_API_ENDPOINT");
		assertExists(endpoint, "missing RIVET_API_ENDPOINT");
		const token = Deno.env.get("RIVET_SERVICE_TOKEN");
		assertExists(token, "missing RIVET_SERVICE_TOKEN");

		this.endpoint = endpoint;

		this.rivetClient = new RivetClient({
			environment: endpoint,
			token,
		});

		this.environment = {
			project: this.ctx.metadata.project.slug,
			environment: this.ctx.metadata.environment.slug,
		};
	}

	static async start(ctx: ActorContext) {
		const manager = new this(ctx);
		await manager.#run();
	}

	async #run() {
		const portStr = Deno.env.get("PORT_HTTP");
		if (!portStr) throw "Missing port";
		const port = parseInt(portStr);
		if (!isFinite(port)) throw "Invalid port";

		const app = new Hono();

		app.use("/*", cors());

		app.get("/rivet/config", (c: HonoContext) => {
			return c.json(
				{
					// HACK(RVT-4376): Replace DNS address used for local dev envs with public address
					endpoint: this.endpoint.replace("rivet-server", "127.0.0.1"),
					project: this.environment.project,
					environment: this.environment.environment,
				} satisfies RivetConfigResponse,
			);
		});

		app.post("/actors", async (c: HonoContext) => {
			// Get actor
			const body: ActorsRequest = await c.req.json();
			const actor = await queryActor(
				this.rivetClient,
				this.environment,
				body.query,
			);

			// Fetch port
			const httpPort = actor.network.ports[PORT_NAME];
			assertExists(httpPort, "missing port");
			const hostname = httpPort.hostname;
			assertExists(hostname);
			const port = httpPort.port;
			assertExists(port);

			let isTls = false;
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

			const path = httpPort.path ?? "";

			const endpoint = `${
				isTls ? "https" : "http"
			}://${hostname}:${port}${path}`;

			return c.json({ endpoint } satisfies ActorsResponse);
		});

		const server = Deno.serve({
			port,
			hostname: "0.0.0.0",
		}, app.fetch);
		await server.finished;
	}
}
