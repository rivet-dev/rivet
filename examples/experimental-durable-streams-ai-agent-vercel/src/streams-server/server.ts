import { DurableStreamTestServer } from "@durable-streams/server";

const PORT = 8787;

const server = new DurableStreamTestServer({
	port: PORT,
	host: "127.0.0.1",
	longPollTimeout: 30_000,
});

const url = await server.start();

console.log(`Durable streams server running at ${url}`);
