// Open a connection by actor name. The selector query parameters are the same
// ones used by the HTTP endpoints; the token and encoding travel as
// WebSocket subprotocols because browsers cannot set headers on an upgrade.
const url = new URL("wss://api.rivet.dev/gateway/counter/connect");
url.searchParams.set("rvt-namespace", process.env.RIVET_NAMESPACE!);
url.searchParams.set("rvt-method", "getOrCreate");
url.searchParams.set("rvt-key", "my-counter");
url.searchParams.set("rvt-pool", "default");

const ws = new WebSocket(url, [
	"rivet",
	"rivet_encoding.json",
	`rivet_token.${process.env.RIVET_TOKEN}`,
]);

// The server sends Init as the first message once the actor is ready.
ws.onmessage = (event) => {
	const { body } = JSON.parse(String(event.data));
	if (body.tag === "Init") {
		console.log(
			"connected to actor",
			body.val.actorId,
			"as",
			body.val.connectionId,
		);
	}
};

export {};
