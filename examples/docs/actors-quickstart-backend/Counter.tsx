import { createRivetKit } from "@rivetkit/react";
import { useState } from "react";
import type { registry } from "./index";

const { useActor } = createRivetKit<typeof registry>("http://localhost:6420");

function Counter() {
	const [count, setCount] = useState(0);

	// Get or create a counter actor for the key "my-counter"
	const counter = useActor({
		name: "counter",
		key: ["my-counter"]
	});

	// Listen to realtime events
	counter.useEvent("newCount", (x: number) => setCount(x));

	const increment = async () => {
		// Call actions
		await counter.connection?.increment(1);
	};

	return (
		<div>
			<p>Count: {count}</p>
			<button onClick={increment}>Increment</button>
		</div>
	);
}
