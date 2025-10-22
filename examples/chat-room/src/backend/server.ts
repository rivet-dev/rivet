import { registry } from "./registry";

registry.start({
	cors: {
		origin: "http://localhost:5173",
		credentials: true,
	},
});
