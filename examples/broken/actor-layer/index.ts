// Without Rivet actor class

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

async function handler(req: Request) {
	console.log("Received request");

  let newCount = (await Rivet.kv.get("count") ?? 0) + 1;
  await Rivet.kv.put("count", newCount);

	return new Response(newCount.toString());
}

// With Rivet actor class

import { Actor } from "@rivet-gg/rivet";

interface State {
  count: number;
}

class Counter extends Actor<State> {
  initialize(): State {
    return { count: 0 };
  }

  async onRequest(req) {
    this.count += 1;
    return new Response(this.count.toString());
  }
}

Rivet.run(Counter);
