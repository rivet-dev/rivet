console.log(Deno.env.toObject());

let server = Deno.serve({
	handler,
	port: parseInt(Deno.env.get("PORT_testing2") ?? Deno.env.get("HTTP_PORT")),
});

await server.finished;

function handler(req) {
	console.log("req");

	return new Response(req.body, {
		status: 200,
		headers: { "Content-Type": "application/json" },
	});
}
