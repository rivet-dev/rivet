import { setup, actor } from "rivetkit";

const myActor = actor({ actions: {} });

const registry = setup({
	use: { myActor },
});
