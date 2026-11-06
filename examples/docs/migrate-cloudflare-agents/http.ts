import { actor } from "rivetkit";

const encoder = new TextEncoder();

export const reportAgent = actor({
	onRequest: (_c, request) => {
		const url = new URL(request.url);

		if (url.pathname === "/status") {
			return Response.json({ status: "ok" });
		}

		// Server-sent events: return a streaming body
		if (url.pathname === "/stream") {
			const stream = new ReadableStream<Uint8Array>({
				async start(controller) {
					for (const word of ["hello", "world"]) {
						controller.enqueue(encoder.encode(`data: ${word}\n\n`));
						await new Promise((resolve) =>
							setTimeout(resolve, 100),
						);
					}
					controller.close();
				},
			});

			return new Response(stream, {
				headers: {
					"Content-Type": "text/event-stream",
					"Cache-Control": "no-cache",
				},
			});
		}

		return new Response("Not found", { status: 404 });
	},

	actions: {},
});
