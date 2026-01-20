// FIXME: Re-enable once inline client is fixed
// import { createInlineClient } from "@rivetkit/cloudflare-workers";
// import { registry } from "./registry";

// const {
// 	client,
// 	fetch: rivetFetch,
// 	ActorHandler,
// } = createInlineClient(registry);

// // IMPORTANT: Your Durable Object must be exported here
// export { ActorHandler };

// export default {
// 	fetch: async (request, env, ctx) => {
// 		const url = new URL(request.url);

// 		// Custom request handler
// 		if (
// 			request.method === "POST" &&
// 			url.pathname.startsWith("/increment/")
// 		) {
// 			const name = url.pathname.slice("/increment/".length);

// 			const counter = client.counter.getOrCreate(name);
// 			const newCount = await counter.increment(1);

// 			return new Response(`New Count: ${newCount}`, {
// 				headers: { "Content-Type": "text/plain" },
// 			});
// 		}

// 		// Optional: If you want to access Rivet Actors publicly, mount the path
// 		if (url.pathname.startsWith("/rivet")) {
// 			const strippedPath = url.pathname.substring("/rivet".length);
// 			url.pathname = strippedPath;
// 			console.log("URL", url.toString());
// 			const modifiedRequest = new Request(url.toString(), request);
// 			return rivetFetch(modifiedRequest, env, ctx);
// 		}

// 		return new Response("Not Found", { status: 404 });
// 	},
// } satisfies ExportedHandler;
