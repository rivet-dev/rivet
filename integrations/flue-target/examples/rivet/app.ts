import { listRuns } from '@flue/runtime';
import { createDefaultFlueApp } from '@flue/runtime/adapter-kit';

const flueApp = createDefaultFlueApp();

export default {
	async fetch(request: Request, env?: unknown): Promise<Response> {
		const url = new URL(request.url);
		if (url.pathname === '/admin/runs') {
			return Response.json(await listRuns({ limit: 100 }));
		}
		return flueApp.fetch(request, env);
	},
};
