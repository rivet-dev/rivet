import { setup, actor } from "rivetkit";

const myActor = actor({ actions: {} });

// Reads from RIVET_ENDPOINT, RIVET_TOKEN, and RIVET_NAMESPACE
const registry = setup({
	use: { myActor },
});
