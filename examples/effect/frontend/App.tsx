import { createRivetKit } from "@rivetkit/react";
import { useEffect, useState } from "react";
import type { registry } from "../src/actors.ts";

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

	counter.useEvent("newCount", (data: { count: number; updatedBy: string }) => {
		setCount(data.count);
	});

	const increment = async (amount: number) => {
		if (counter.connection) {
			await counter.connection.increment(amount);
		}
	};

	const decrement = async (amount: number) => {
		if (counter.connection) {
			await counter.connection.decrement(amount);
		}
	};

	const reset = async () => {
		if (counter.connection) {
			await counter.connection.reset();
		}
	};

	const batchIncrement = async () => {
		if (counter.connection) {
			await counter.connection.batchIncrement([1, 2, 3, 4, 5]);
		}
	};

	return (
		<div className="counter-app">
			<div className="counter-container">
				<div className="counter-header">
					<h1>Effect Counter</h1>
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
						onClick={() => decrement(1)}
						disabled={!counter.connection}
						className="counter-button secondary"
					>
						-1
					</button>
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

				<div className="counter-controls">
					<button
						onClick={batchIncrement}
						disabled={!counter.connection}
						className="counter-button accent"
					>
						Batch +15
					</button>
					<button
						onClick={reset}
						disabled={!counter.connection}
						className="counter-button danger"
					>
						Reset
					</button>
				</div>

				<div className="info-box">
					<p>This example uses <strong>@rivetkit/effect</strong> for all actor logic.</p>
					<p>Actions use Effect.gen for composable, type-safe operations with structured logging.</p>
				</div>
			</div>
		</div>
	);
}
