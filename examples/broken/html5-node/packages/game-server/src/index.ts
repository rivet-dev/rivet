import { Rivet } from "rivet-sdk";
import { Server as WebSocketServer, WebSocket } from "ws";

const LOBBY_ID = process.env.LOBBY_ID ?? "00000000-0000-0000-0000-000000000000";
const LOBBY_TOKEN = process.env.LOBBY_TOKEN;

const rivet = new Rivet();

// Setup WebSocket server
const wss = new WebSocketServer({ port: parseInt(process.env.PORT!) || 7777 });
console.log(`WebSocket server listening on port ${wss.options.port}`);

// Handle connections
const connectedClients: WebSocket[] = [];
wss.on("connection", async (ws: WebSocket, req: any) => {
	connectedClients.push(ws);

	const searchParams = new URL(req.url!, `ws://${req.headers.host}`).searchParams;
	const playerToken = searchParams.get("token");

	ws.on("close", async () => {
		// Remove client from connectedClients array
		const index = connectedClients.indexOf(ws);
		if (index > -1) {
			connectedClients.splice(index, 1);
		}

		// Rivet disconnection
		try {
			await rivet.lobbies.setPlayerDisconnected({
				lobbyId: LOBBY_ID,
				lobbyToken: LOBBY_TOKEN,
				playerTokens: [playerToken],
				client: ws,
			});
		} catch (err) {
			console.error("Failed to disconnect player", err);
		}
	});

	// Rivet connection
	try {
		await rivet.lobbies.setPlayerConnected({
			lobbyId: LOBBY_ID,
			lobbyToken: LOBBY_TOKEN,
			playerTokens: [playerToken],
		});
	} catch (err) {
		console.error("Failed to connect player", err);
		ws.close();
		return;
	}

	ws.on("message", (rawData: string) => {
		let [event, data] = JSON.parse(rawData);
		if (event === "ping") {
			ws.send(JSON.stringify(["pong", data]));
		}
	});

	// Send init
	ws.send(JSON.stringify(["init", { publicIp: req.headers["x-forwarded-for"] } ]));

	// Send current counter value immediately upon connection
	ws.send(JSON.stringify(["counter", globalCounter]));
});


// Broadcast counter to all clients
let globalCounter = 0;
function broadcastCounter() {
    connectedClients.forEach((client) => {
		client.send(JSON.stringify(["counter", globalCounter]));
    });
}

setInterval(() => {
    globalCounter++;
    broadcastCounter();
}, 1000);

// Ready to start accepting players
rivet.lobbies.setLobbyReady({ lobbyId: LOBBY_ID, lobbyToken: LOBBY_TOKEN })
	.then(() => console.log("Lobby ready"))
	.catch((err) => {
		console.error("Failed to start lobby", err);
		process.exit(1);
	});
