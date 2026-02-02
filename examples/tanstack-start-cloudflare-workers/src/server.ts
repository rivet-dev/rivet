import { createHandler } from "@rivetkit/cloudflare-workers";
import handler, { createServerEntry } from "@tanstack/react-start/server-entry";
import { registry } from "./actors";

const { handler: rivetKitHandler, ActorHandler } = createHandler(registry);

const serverEntry = createServerEntry({
	async fetch(request) {
		const url = new URL(request.url);
		// Route requests to RivetKit or TanStack React Start based on the path
		if (url.pathname.startsWith("/api/rivet")) {
			return await rivetKitHandler.fetch(request);
		}
		return await handler.fetch(request);
	},
});

export { ActorHandler };

export default {
	...serverEntry,
};
