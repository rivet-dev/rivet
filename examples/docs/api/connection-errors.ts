const ws = new WebSocket(
	`wss://api.rivet.dev/gateway/${process.env.ACTOR_ID}/connect`,
	["rivet", "rivet_encoding.json", `rivet_token.${process.env.RIVET_TOKEN}`],
);

ws.onmessage = (event) => {
	const { body } = JSON.parse(String(event.data));
	if (body.tag !== "Error") return;

	const { group, code, message, actionId } = body.val;
	if (actionId !== null) {
		// The error answers the ActionRequest with this id.
		console.error(
			`action ${actionId} failed: ${group}.${code}: ${message}`,
		);
	} else {
		// Connection-level error, for example a rejected message.
		console.error(`connection error: ${group}.${code}: ${message}`);
	}
};

// Errors that reject the upgrade itself arrive as a close frame instead,
// with a reason of the form "{group}.{code}#{ray_id}".
ws.onclose = (event) => {
	if (event.code !== 1000) console.error("closed:", event.code, event.reason);
};

export {};
