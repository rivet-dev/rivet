import { Server } from "socket.io";
import http from "http";
import { RivetClient } from "@rivet-gg/api";

console.log(process.env);

async function main() {
	let client = new RivetClient({
		token: process.env.RIVET_TOKEN,
	});
	let connections = new Map();

	// Create server
	let port = process.env["PORT_test-bridge"];
	let httpServer = http.createServer().listen(3000);
	let io = new Server(httpServer);
	io.on("connection", (socket) => {
		connections.set(socket.id, new Conn(socket));
	});

	// Mark ready
	await client.matchmaker.lobbies.ready();
	console.log("ready");
}
await main();

class Conn {
	// connected: boolean = false;
	// playerToken: string;

	constructor(socket) {
		this.socket = socket;

		console.log(`${this.socket.id} connect`);

		socket.on(0, this.connect.bind(this));

		socket.on("disconnect", (reason) => {
			this.disconnect(reason);
		});

		socket.emit(0, socket.id);
	}

	async connect(playerToken) {
		console.log(`${this.socket.id} init`);

		this.playerToken = playerToken;

		try {
			await client.matchmaker.players.connected({ playerToken: this.playerToken });
			this.connected = true;

			console.log(`${this.socket.id} rivet connect`);
		} catch (e) {
			console.error("failed to connect to rivet:");
			console.error(e);

			this.disconnect("error");
		}
	}

	async disconnect(reason) {
		if (this.connected) {
			try {
				await client.matchmaker.players.disconnected({ playerToken: this.playerToken });

				console.log(`${this.socket.id} rivet disconnect`);
			} catch (e) {
				console.error("failed to disconnect from rivet:");
				console.error(e);
			}
		}

		console.log(`${this.socket.id} disconnect`, reason);

		this.socket.disconnect(reason);
	}
}
