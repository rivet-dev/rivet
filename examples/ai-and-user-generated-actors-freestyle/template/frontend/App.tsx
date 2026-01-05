import { createRivetKit } from "@rivetkit/react";
import { useEffect, useState } from "react";
import type { registry } from "../backend/registry";

console.log("Environment variables:", {
	VITE_RIVET_ENDPOINT: import.meta.env.VITE_RIVET_ENDPOINT,
	VITE_RIVET_NAMESPACE: import.meta.env.VITE_RIVET_NAMESPACE,
	VITE_RIVET_TOKEN: import.meta.env.VITE_RIVET_TOKEN,
});

const { useActor } = createRivetKit<typeof registry>({
	endpoint: import.meta.env.VITE_RIVET_ENDPOINT,
	namespace: import.meta.env.VITE_RIVET_NAMESPACE,
	token: import.meta.env.VITE_RIVET_TOKEN,
});

export function App() {
	const [count, setCount] = useState(0);

	const counter = useActor({
		name: "counter",
		key: ["global"],
	});

	useEffect(() => {
		if (counter.connection) {
			counter.connection.getCount().then(setCount);
		}
	}, [counter.connection]);

	counter.useEvent("countChanged", (newCount: number) => {
		setCount(newCount);
	});

	const increment = async () => {
		if (counter.connection) {
			const newCount = await counter.connection.increment();
			setCount(newCount);
		}
	};

	const decrement = async () => {
		if (counter.connection) {
			const newCount = await counter.connection.decrement();
			setCount(newCount);
		}
	};

	return (
		<div className="counter-container">
			<h1>Realtime Counter</h1>
			<div className="status" style={{ marginBottom: "16px" }}>
				Try opening another tab
			</div>
			<div className="counter-display">
				<h2>{count}</h2>
			</div>
			<div className="counter-buttons">
				<button onClick={decrement} disabled={!counter.connection}>
					-
				</button>
				<button onClick={increment} disabled={!counter.connection}>
					+
				</button>
			</div>
			<div className="status">
				{counter.connection ? "Connected" : "Connecting..."}
			</div>
		</div>
	);
}
