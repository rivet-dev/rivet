const ws = new WebSocket(
	`wss://api.rivet.dev/gateway/${process.env.ACTOR_ID}/connect`,
	["rivet", "rivet_encoding.json", `rivet_token.${process.env.RIVET_TOKEN}`],
);

ws.onopen = () => {
	// Subscribe once. The server pushes an Event message every time the actor
	// broadcasts or sends countChanged to this connection.
	ws.send(
		JSON.stringify({
			body: {
				tag: "SubscriptionRequest",
				val: { eventName: "countChanged", subscribe: true },
			},
		}),
	);
};

ws.onmessage = (event) => {
	const { body } = JSON.parse(String(event.data));
	if (body.tag === "Event" && body.val.name === "countChanged") {
		const [count] = body.val.args;
		console.log("count is now", count);
	}
};

export {};
