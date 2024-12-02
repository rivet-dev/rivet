// Connection methods:
// - HTTP
//  - Req/Res
// - WebSocket
//  - Req/Res
//  - Bidirectional
//
// Prior art:
// - DO RPC
// - gRPC
// - PartyKit connections
// - socket.io
// - tRPC
//
// Stateful vs stateless:
// - Stateful is more common for realtime
//  - PartyKit
//  - socket.io
//  - gRPC
// - Stateless can be added later
//
// Features:
// - State
// - Timers
// - Cron
// - Lifecycle (shutdown)
// - Metadata (tags)
// - Bidirectional streaming
// - Broadcasting?
// - Events? (for broadcasting)

interface State {
	count: number;
}

class Counter extends Actor<State> {
	onConnection(connection: ActorConnection) {
	}

	onRequest(request: ActorRequest): Response {
	}
}

// ===

class Counter extends Actor<State> {
	initialize(): State {
		return { count: 0 };
	}

	clientStreamingRpc(stream: ClientStream<number>): number {
		while (true) {
			let x = await countStream.recv();
		}
	}

	serverStreamingRpc(stream: ServerStream<number, number>) {
		setInterval(() => {
			stream.send(stream.body * 2);
		}, 1000);
	}

	bidirectionalStreamingRpc(stream: Socket<number, number>) {
		while (true) {
			const x = await stream.recv();
			stream.send(x * 2);
		}
	}

	async increment(count: number): number {
	}
}

// ===

class Counter extends Actor<State> {
	initialize(): State {
		return { count: 0 };
	}

	onConnection(conn: Connection) {
		conn.on("increment", () => {
			this.state.count += 1;
			this.boradcast(this.state.count);
		});

		conn.on("close", () => {
			this.broadcast("closed");
		});
	}

	increment() {
	}

	decrement() {
	}
}

// ===

class Counter extends Actor<State> {
	constructor() {
		super();

		this.on("increment", () => {
			this.state.connections += 1;
			this.state.count += 1;
			this.broadcast(this.state);
		});

		this.on("close", () => {
			this.state.connections -= 1;
			this.broadcast(this.state);
		});
	}

	initialize(): State {
		return { count: 0 };
	}

	onConnection(conn: Connection) {
		conn.on("increment", () => {
			this.state.count += 1;
			this.boradcast(this.state.count);
		});

		conn.on("close", () => {
			this.broadcast("closed");
		});
	}
}

// ===

class Counter extends Actor<State> {
	initialize(): State {
		return { count: 0 };
	}

	connect(conn: Socket<number, number>) {
		this.state.connections += 1;

		conn.on("incr", () => {
			this.state.count += 1;
			this.boradcast(this.state);
		});
		conn.on("close", () => {
			this.state.connections -= 1;
			this.boradcast(this.state);
		});
	}

	getState() {
	}

	increment(): number {
		this.state.count += 1;
	}
}

// ===

class Counter extends Actor<State> {
	initialize(): State {
		return { count: 0 };
	}

	// Simple unary RPC
	async increment(count: number): Promise<number> {
		this.state.count += count;
		return this.state.count;
	}

	// Client streaming - receives multiple numbers, returns final sum
	async clientStreamingRpc(stream: ClientStream<number>): Promise<number> {
		while (true) {
			const value = await stream.next();
			if (value === null) break;

			this.state.count += value;
		}
		return this.state.count;
	}

	// Server streaming - multiplies input by 2 and streams result periodically
	serverStreamingRpc(stream: ServerStream<number, number>) {
		setInterval(() => {
			stream.send(stream.body * 2);
		}, 1000);
	}

	// Bidirectional streaming - multiplies each received number by 2 and sends it back
	async bidirectionalStreamingRpc(stream: BidirectionalStream<number, number>) {
		while (true) {
			const value = await stream.read();
			if (value === null) break;

			stream.send(value * 2);
		}
	}
}

// ===

// Inspiration:
// - Simplicity of Socket.io
// - Rooms from Socket.io & PartyKit
// - Hybrid approach of gRPC
// - Simplicity of RPC of tRPC and Durable Objects

// TODO: Focus on how you'd do it in raw JS
// Is presence a thing?

class Counter extends Actor<State> {
	initialize(): State {
		return { count: 0 };
	}

	async increment(count: number): Promise<number> {
		this.state.count += count;
		return this.state.count;
	}

	async observe(socket: Socket<void, number>) {
		socket.onMessage("foo", (x) => {
			console.log(x);
			this.network.broadcast("observe", 5);
			socket.close();
		});

		this.on("stateUpdate", () => {
			this.socket.send("state", this.state);
		});

		socket.onClose(() => {
			console.log("close");
		});
	}
}

// ===

// TODO: Some sort of version management?

class Counter extends Actor<State> {
	initialize(): State {
		return { count: 0 };
	}

	async increment(count: number): Promise<number> {
		this.state.count += count;
		return this.state.count;
	}

	async observe(socket: Socket<void, number>) {
		this.onStateUpdate((state) => {
			this.socket.send("state", state);
		});

		socket.onMessage("foo", (x) => {
			console.log(x);
			this.network.broadcast("observe", 5);
			socket.close();
		});

		socket.onClose(() => {
			console.log("close");
		});
	}

	// async observe(socket: Socket<number, number>): Promise<any> {
	// 	return {
	// 		increment(count) {
	// 			this.state.count += count;
	// 		}
	// 	}
	// }

	// async realtime(socket: Socket, x: number): Promise<void> {
	// 	if (await authenticate()) {
	// 		throw new ForbiddenError();
	// 	}
	//
	// 	socket.on("increment", (y: number) => {
	// 		this.state.count += x * y;
	// 		socket.send("new count", this.state.count);
	// 	});
	//
	// 	socket.onClose(() => {
	//
	// 	});
	// }
	//
	// async realtime(socket: Socket, x: number): Promise<void> {
	// 	const unsubscribe = this.onStateChange((state) => {
	// 		if (state.count % x == 0) {
	// 			socket.send("new count", this.state.count);
	// 		}
	// 	});
	//
	// 	socket.onClose(() => {
	// 		unsubscribe();
	// 	});
	// }
}

// ===

// Session = vague
// Connection = ok?
// Client = does not imply stateful

// HTTP requires stateless
// WebSocket requires handshake which is slow
// We can have optional state for clients, but this requires a connect call

interface ConnectionData {
	n: number;
}

class Counter extends Actor<State, ConnectionData> {
	initializeState(): State {
		return { count: 0 };
	}

	// onStart() {
	// 	this.onStateChange((state) => {
	// 		this.broadcast("new count", state.count);
	// 		// for (const socket of this.sockets) {
	// 		// 	socket.send("new count", state.count);
	// 		// }
	// 	});
	// }
	//
	// onBeforeConnect(session) {
	// 	if (!await authenticate(session)) throw new Forbidden();
	// }
	//
	// onConnect(c) {
	//
	// }
	//
	// onDisconnect(c) {
	//
	// }

	async increment(c, count: number): Promise<number> {
		this.state.count += count;
		return this.state.count;
	}
}

