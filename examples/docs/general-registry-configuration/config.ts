import { setup, actor } from "rivetkit";

const myActor = actor({ actions: {} });

const registry = setup({
	use: { myActor },
	endpoint: "https://rivet.example.com",
	token: process.env.RIVET_TOKEN,
	namespace: "production",
});
