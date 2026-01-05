import { createRivetKit } from "@rivetkit/react";
import { useState } from "react";
import type { registry } from "../src/registry";

const { useActor } = createRivetKit<typeof registry>(`${window.location.origin}/api/rivet`);

function App() {
	const [count, setCount] = useState(0);
	const [counterName, setCounterName] = useState("test-counter");

	const counter = useActor({
		name: "counter",
		key: [counterName],
	});

	counter.useEvent("newCount", (x: number) => setCount(x));

	const increment = async () => {
		await counter.connection?.increment(1);
	};

	return (
		<div>
			<h1>Counter: {count}</h1>
			<input
				type="text"
				value={counterName}
				onChange={(e) => setCounterName(e.target.value)}
				placeholder="Counter name"
			/>
			<button onClick={increment}>Increment</button>
		</div>
	);
}

export default App;
