import { registry } from "./registry.js";

registry.start({
	cors: {
		origin: "http://localhost:5173",
		credentials: true,
	},
});
