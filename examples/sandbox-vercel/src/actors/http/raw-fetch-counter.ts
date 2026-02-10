import { Hono } from "hono";
import { type ActorContextOf, actor } from "rivetkit";

export const rawFetchCounter = actor({
	state: {
		count: 0,
	},
	createVars: () => {
		// Setup router
		return { router: createCounterRouter() };
	},
	onRequest: (c, request) => {
		return c.vars.router.fetch(request, { actor: c });
	},
	actions: {
		// ...actions...
	},
});

function createCounterRouter(): Hono<any> {
	const app = new Hono<{
		Bindings: { actor: ActorContextOf<typeof rawFetchCounter> };
	}>();

	app.get("/count", (c) => {
		const { actor } = c.env;

		return c.json({
			count: actor.state.count,
		});
	});

	app.post("/increment", (c) => {
		const { actor } = c.env;

		actor.state.count++;
		return c.json({
			count: actor.state.count,
		});
	});

	return app;
}
