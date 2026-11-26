import { Client } from "@rivetkit/effect";

export const ClientLayer = Client.layer({
	endpoint: "https://api.rivet.dev",
	namespace: "my-namespace",
	token: "<scoped-token>",
	poolName: "my-pool",
	headers: { "x-custom": "value" },
});
