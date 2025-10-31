import { createRivetKit } from "@rivetkit/react";
import { useState } from "react";
import type { registry } from "../backend/registry";

const { useActor } = createRivetKit<typeof registry>("http://localhost:6420");

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
