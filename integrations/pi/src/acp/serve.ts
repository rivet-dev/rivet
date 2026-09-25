import { Readable, Writable } from "node:stream";
import { AgentSideConnection, ndJsonStream } from "@agentclientprotocol/sdk";
import { PiAcpAgent, type PiAcpOptions } from "./agent.js";

/**
 * Serves ACP on stdin and stdout until the editor closes stdin. ACP sends
 * JSON-RPC on stdout and RivetKit writes its logs to stdout, so every other
 * stdout write goes to stderr from the moment this runs. Call it before
 * creating RivetKit clients.
 */
export function serveAcp(options: PiAcpOptions): void {
	const writeProtocol = process.stdout.write.bind(process.stdout);
	process.stdout.write = process.stderr.write.bind(process.stderr) as typeof process.stdout.write;

	const input = Writable.toWeb(
		new Writable({
			write(chunk, _encoding, callback) {
				writeProtocol(chunk, callback);
			},
		}),
	);
	const output = Readable.toWeb(process.stdin) as ReadableStream<Uint8Array>;

	let agent: PiAcpAgent | undefined;
	new AgentSideConnection((conn) => {
		agent = new PiAcpAgent(conn, options);
		return agent;
	}, ndJsonStream(input, output));

	const shutdown = async () => {
		await agent?.dispose().catch(() => {});
		process.exit(0);
	};
	process.stdin.on("end", shutdown);
	process.on("SIGINT", shutdown);
	process.on("SIGTERM", shutdown);
}
