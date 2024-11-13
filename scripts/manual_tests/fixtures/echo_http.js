console.log(Deno.env.toObject());

const port = Deno.env.get("PORT_ds_http");
if (!port) throw new Error("missing PORT_ds_http");

console.log(`Starting server on ${port}`);
let server = Deno.serve({
	handler,
	port: parseInt(port),
});

await server.finished;

function handler(req) {
	console.log("req");

	return new Response(req.body, {
		status: 200,
		headers: { "Content-Type": "application/json" },
	});
}
