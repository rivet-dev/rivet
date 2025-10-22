import { registry } from "./registry";

registry.start({
	cors: {
		origin: "http://localhost:3000",
		credentials: true,
	},
});
