console.log(Deno.env.toObject());

export default {
	async start(ctx) {
		let server = Deno.serve({
			handler,
			port: parseInt(
				Deno.env.get("PORT_DS_TESTING2") ?? Deno.env.get("HTTP_PORT"),
			),
		});

		await server.finished;
	},
};

function handler(req) {
	console.log("req", req);

	let url = new URL(req.url);

	if (url.pathname == "/exit") Deno.exit(parseInt(req.body));

	return new Response(req.body, {
		status: 200,
		headers: { "Content-Type": "application/json" },
	});
}
