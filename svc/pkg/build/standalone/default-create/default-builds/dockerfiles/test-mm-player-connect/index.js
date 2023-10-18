import net from "net";
import { RivetClient } from "@rivet-gg/api";

console.log(process.env);

const PORT = process.env.PORT;

let client = new RivetClient({
	token: process.env.RIVET_TOKEN,
	environment: {
		matchmaker: `${process.env.RIVET_API_ENDPOINT}/matchmaker`,
	},
});
let connectionId = 0;
let connections = new Map();

// Create server
let server = net.createServer();

server.on("listening", async () => {
	console.log("listening on", PORT);

	try {
		await client.matchmaker.lobbies.ready({});
		console.log("ready");
	} catch (e) {
		console.error(e);
		server.close();
	}
});

server.on("connection", (socket) => {
	let id = connectionId++;
	connections.set(id, new Conn(socket, id));
});

server.on("error", (e) => {
	console.error(e);
});

server.on("close", async () => {
	console.log("closing server");
});

server.listen(PORT);

class Conn {
	constructor(socket, id) {
		this.socket = socket;
		this.id = id;
		this.playerToken = null;
		this.connected = false;
		this.start = process.hrtime.bigint();

		console.log(`${this.id} connect`);

		this.socket.on("data", this.onData.bind(this));

		this.socket.on("error", () => {
			console.error("socket error:");
			console.error(err);

			this.disconnect(err.message);
		});

		this.socket.on("close", () => {
			this.disconnect("close");
		});

		this.socket.on("end", () => {
			this.disconnect("client disconnect");
		});

		// Send ID
		let buffer = Buffer.allocUnsafe(4);
		buffer.writeUInt32LE(this.id, 0);
		this.socket.write(buffer);
	}

	async onData(data) {
		if (this.connected) {
			console.log(`${this.id} data`, data.toString());
			console.log(process.hrtime.bigint() - this.start);

			this.socket.write(data);
		} else {
			this.playerToken = data.toString();
			console.log(`${this.id} init`, this.playerToken);

			try {
				await client.matchmaker.players.connected({ playerToken: this.playerToken });
				this.connected = true;

				console.log(`${this.id} rivet connect`);
			} catch (e) {
				console.error("failed to connect to rivet:");
				console.error(e);

				this.disconnect("failed auth");
			}
		}
	}

	async disconnect(reason) {
		if (this.connected) {
			this.connected = false;

			console.log(`${this.id} disconnect`, reason);
			console.log(process.hrtime.bigint() - this.start);

			try {
				await client.matchmaker.players.disconnected({ playerToken: this.playerToken });

				console.log(`${this.id} rivet disconnect`);
			} catch (e) {
				console.error("failed to disconnect from rivet:");
				console.error(e);
			}
		}

		this.socket.destroy();
		connections.delete(this.id);
	}
}
