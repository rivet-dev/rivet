import { createRivetKit } from "@rivetkit/react";
import { useEffect, useState } from "react";
import type { registry } from "../server/actors.ts";

const { useActor } = createRivetKit<typeof registry>(`${location.origin}/api/rivet`);

export function App() {
	const [counterId, setCounterId] = useState("default");
	const [count, setCount] = useState<number>(0);

	const counter = useActor({
		name: "counter",
		key: [counterId],
	});

	useEffect(() => {
		if (counter.connection) {
			counter.connection.getCount().then(setCount);
		}
	}, [counter.connection]);

	counter.useEvent("newCount", (newCount: number) => {
		setCount(newCount);
	});

	const increment = async (amount: number) => {
		if (counter.connection) {
			await counter.connection.increment(amount);
		}
	};

	return (
		<div className="counter-app">
			<div className="counter-container">
				<div className="counter-header">
					<h1>Hello World</h1>
					<div className={`status-indicator ${counter.connection ? 'connected' : 'disconnected'}`}>
						<div className="status-dot"></div>
						<span>{counter.connection ? 'Connected' : 'Connecting...'}</span>
					</div>
				</div>

				<div className="counter-settings">
					<div className="setting-group">
						<label htmlFor="counterId">Counter ID</label>
						<input
							id="counterId"
							type="text"
							value={counterId}
							onChange={(e) => setCounterId(e.target.value)}
							placeholder="Enter counter ID"
							className="setting-input"
						/>
					</div>
				</div>

				<div className="counter-display">
					<div className="count-value">{count}</div>
					<p className="count-label">Current Count</p>
				</div>

				<div className="counter-controls">
					<button
						onClick={() => increment(1)}
						disabled={!counter.connection}
						className="counter-button"
					>
						+1
					</button>
					<button
						onClick={() => increment(5)}
						disabled={!counter.connection}
						className="counter-button"
					>
						+5
					</button>
					<button
						onClick={() => increment(10)}
						disabled={!counter.connection}
						className="counter-button"
					>
						+10
					</button>
				</div>

				<div className="info-box">
					<p>This counter is shared across all clients using the same Counter ID.</p>
					<p>Try opening this page in multiple tabs or browsers!</p>
				</div>
			</div>
		</div>
	);
}
