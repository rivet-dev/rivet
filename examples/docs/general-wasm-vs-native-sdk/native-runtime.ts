import { actor, setup } from "rivetkit";

const counter = actor({ actions: {} });

const registry = setup({ use: { counter } });
