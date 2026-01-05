import express from "express";
import { createClient } from "rivetkit/client";
import { registry } from "./registry";

// Start RivetKit
registry.startRunner();
const client = createClient<typeof registry>();

// Setup router
const app = express();

// Example HTTP endpoint
app.post("/increment/:name", async (req, res) => {
	const name = req.params.name;

	const counter = client.counter.getOrCreate(name);
	const newCount = await counter.increment(1);

	res.send(`New Count: ${newCount}`);
});

app.listen(8080, () => {
	console.log("Listening at http://localhost:8080");
});

export default app;
