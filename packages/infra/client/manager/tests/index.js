export default {
	async start(ctx) {
		console.log(Deno.env.toObject());

		console.log(ctx.metadata);

		let server = Deno.serve({
			handler,
			port: parseInt(Deno.env.get("PORT_MAIN")),
		});

		await server.finished;
	},
};

function handler(req) {
	console.log("req");

	return new Response(req.body, {
		status: 200,
		headers: { "Content-Type": "application/json" },
	});
}
