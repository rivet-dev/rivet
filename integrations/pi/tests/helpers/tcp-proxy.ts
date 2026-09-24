import { type AddressInfo, connect, createServer, type Socket } from "node:net";

/**
 * A TCP proxy in front of the local engine. Tests take it down to drop a
 * client's sockets while the actor keeps running on the other side.
 */
export interface TcpProxy {
	/** The proxy's address, used as the client endpoint. */
	url: string;
	/** Drops every open connection and refuses new ones until `up()`. */
	down(): void;
	/** Forwards new connections again. */
	up(): void;
	close(): Promise<void>;
}

export async function startTcpProxy(target: string): Promise<TcpProxy> {
	const { hostname, port: targetPort } = new URL(target);
	const sockets = new Set<Socket>();
	let refusing = false;

	const track = (socket: Socket) => {
		sockets.add(socket);
		socket.on("close", () => sockets.delete(socket));
		socket.on("error", () => {});
	};
	const server = createServer((client) => {
		if (refusing) {
			client.destroy();
			return;
		}
		const upstream = connect(Number(targetPort), hostname);
		track(client);
		track(upstream);
		client.pipe(upstream).pipe(client);
		client.on("close", () => upstream.destroy());
		upstream.on("close", () => client.destroy());
	});
	await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
	const { port } = server.address() as AddressInfo;

	return {
		url: `http://127.0.0.1:${port}`,
		down: () => {
			refusing = true;
			for (const socket of sockets) socket.destroy();
		},
		up: () => {
			refusing = false;
		},
		close: async () => {
			for (const socket of sockets) socket.destroy();
			await new Promise((resolve) => server.close(resolve));
		},
	};
}
