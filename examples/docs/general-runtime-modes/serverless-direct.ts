import { actor, setup } from "rivetkit";

const myActor = actor({ state: {}, actions: {} });
const registry = setup({ use: { myActor } });

export default registry.serve();
