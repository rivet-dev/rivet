import express from "express";
import { registry } from "./registry";

// Start RivetKit
const { client } = registry.start();

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
	console.log("Listening at http://localhost:6420");
});

export default app;
