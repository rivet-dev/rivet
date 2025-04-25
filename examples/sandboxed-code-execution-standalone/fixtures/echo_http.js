console.log(Deno.env.toObject());

export default {
	async start() {
		const server = Deno.serve({
			handler,
			port: Number.parseInt(Deno.env.get("PORT_HTTP")),
			hostname: "0.0.0.0",
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
