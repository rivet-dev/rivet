const ws = new WebSocket(
	`wss://api.rivet.dev/gateway/${process.env.ACTOR_ID}/connect`,
	["rivet", "rivet_encoding.json", `rivet_token.${process.env.RIVET_TOKEN}`],
);

// Pick a unique id per request. The ActionResponse (or Error) that answers
// it carries the same id, so several actions can be in flight at once.
let nextId = 1;
const pending = new Map<number, (output: unknown) => void>();

function callAction(name: string, args: unknown[]): Promise<unknown> {
	const id = nextId++;
	ws.send(
		JSON.stringify({
			body: { tag: "ActionRequest", val: { id, name, args } },
		}),
	);
	return new Promise((resolve) => pending.set(id, resolve));
}

ws.onmessage = (event) => {
	const { body } = JSON.parse(String(event.data));
	if (body.tag === "ActionResponse") {
		pending.get(body.val.id)?.(body.val.output);
		pending.delete(body.val.id);
	}
};

ws.onopen = async () => {
	const count = await callAction("increment", [1]);
	console.log(count); // 1
};

export {};
