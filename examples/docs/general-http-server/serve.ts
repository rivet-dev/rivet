import { actor, setup } from "rivetkit";

const myActor = actor({ actions: {} });
const registry = setup({ use: { myActor } });

export default registry.serve();
