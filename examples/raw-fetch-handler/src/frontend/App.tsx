import { useState, useEffect } from "react";
import { createClient } from "@rivetkit/react";
import type { registry } from "../backend/registry";

// Create a client that connects to the running server
const client = createClient<typeof registry>("http://localhost:8080");

function Counter({ name }: { name: string }) {
	const [count, setCount] = useState<number | null>(null);
	const [loading, setLoading] = useState(false);

	const actor = client.counter.getOrCreate([name]);

	const fetchCount = async () => {
		const response = await actor.fetch("/count");
		const data = await response.json();
		setCount(data.count);
	};

	const handleIncrement = async () => {
		setLoading(true);
		try {
			// Method 1: Using fetch API
			const response = await actor.fetch("/increment", { method: "POST" });
			const data = await response.json();
			setCount(data.count);
		} finally {
			setLoading(false);
		}
	};

	const handleForwardIncrement = async () => {
		setLoading(true);
		try {
			// Method 2: Using the forward endpoint
			const response = await fetch(`http://localhost:8080/forward/${name}/increment`, {
				method: "POST",
			});
			const data = await response.json();
			setCount(data.count);
		} finally {
			setLoading(false);
		}
	};

	useEffect(() => {
		fetchCount();
	}, []);

	return (
		<div>
			<h2>{name}</h2>
			<p>Count: {count !== null ? count : "Loading..."}</p>
			
			<h3>Via Actor Fetch</h3>
			<button onClick={handleIncrement} disabled={loading}>
				Increment
			</button>
			
			<h3>Via Forward Endpoint</h3>
			<button onClick={handleForwardIncrement} disabled={loading}>
				Increment
			</button>
			
			<br />
			<button onClick={fetchCount} disabled={loading}>
				Refresh
			</button>
			<hr />
		</div>
	);
}

function App() {
	const [counters, setCounters] = useState(["counter-1", "counter-2"]);
	const [newCounterName, setNewCounterName] = useState("");

	const addCounter = () => {
		if (newCounterName && !counters.includes(newCounterName)) {
			setCounters([...counters, newCounterName]);
			setNewCounterName("");
		}
	};

	return (
		<div>
			<h1>RivetKit Raw Fetch Handler Example</h1>
			
			<div>
				<input
					type="text"
					value={newCounterName}
					onChange={(e) => setNewCounterName(e.target.value)}
					placeholder="Counter name"
					onKeyPress={(e) => e.key === "Enter" && addCounter()}
				/>
				<button onClick={addCounter}>Add Counter</button>
			</div>
			
			<hr />
			
			<div>
				{counters.map((name) => (
					<Counter key={name} name={name} />
				))}
			</div>
		</div>
	);
}

export default App;
