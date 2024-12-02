import { randomIntegerBetween, randomSeeded } from "@std/random";

console.log("Hello, world!");
console.log(Deno.env.toObject());

const portStr = Deno.env.get("PORT_http") ?? Deno.env.get("HTTP_PORT");
if (!portStr) throw "Missing port";
const port = parseInt(portStr);
if (!isFinite(port)) throw "Invalid port";

const server = Deno.serve({
	handler,
	port,
	hostname: "0.0.0.0",
});

await server.finished;

const prng = randomSeeded(1n);

function handler(req: Request) {
	console.log("Received request");

	return new Response(randomIntegerBetween(1, 100, { prng }).toString(), {
		status: 200,
		headers: { "Content-Type": "application/json" },
	});
}
