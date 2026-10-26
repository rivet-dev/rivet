"use client";

import { createRivetKit } from "@rivetkit/next-js/client";
import { useState } from "react";
import type { registry } from "./registry";

const { useActor } = createRivetKit<typeof registry>({
	endpoint: process.env.NEXT_PUBLIC_RIVET_ENDPOINT ?? "http://localhost:6420",
});

export function Counter() {
	const [count, setCount] = useState(0);
	const counter = useActor({ name: "counter", key: ["my-counter"] });

	counter.useEvent("newCount", (value: number) => setCount(value));

	return (
		<div>
			<p>Count: {count}</p>
			<button onClick={() => counter.connection?.increment(1)}>Increment</button>
		</div>
	);
}
