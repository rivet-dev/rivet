import { actor, setup } from "rivetkit";

const counter = actor({ state: { count: 0 }, actions: {} });

const registry = setup({ use: { counter } });
