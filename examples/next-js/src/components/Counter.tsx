"use client";

import { createRivetKit } from "@rivetkit/next-js/client";
import { useEffect, useState } from "react";
import type { registry } from "../rivet/actors";

export const { useActor } = createRivetKit<typeof registry>({
	endpoint: process.env.NEXT_PUBLIC_RIVET_ENDPOINT ?? "http://localhost:3000/api/rivet",
	namespace: process.env.NEXT_PUBLIC_RIVET_NAMESPACE,
	token: process.env.NEXT_PUBLIC_RIVET_TOKEN,
});

export function Counter() {
	const [counterId, setCounterId] = useState("default");
	const [count, setCount] = useState<number>(0);

	const counter = useActor({
		name: "counter",
		key: [counterId],
	});

	// Use connStatus from the hook instead of tracking connection state manually
	const isConnected = counter.connStatus === "connected";

	useEffect(() => {
		if (counter.connection && isConnected) {
			counter.connection
				.getCount()
				.then((value) => setCount(value));
		}
	}, [counter.connection, isConnected]);

	counter.useEvent("newCount", (newCount: number) => {
		setCount(newCount);
	});

	const increment = async (amount: number) => {
		if (counter.connection) {
			const nextCount = await counter.connection.increment(amount);
			setCount(nextCount);
		}
	};

	return (
		<div className="counter-app">
			<div className="counter-container">
				<div className="counter-header">
					<h1>Counter Demo</h1>
					<div className={`status-indicator ${isConnected ? 'connected' : 'disconnected'}`}>
						<div className="status-dot"></div>
						<span>{isConnected ? 'Connected' : 'Disconnected'}</span>
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
						disabled={!isConnected}
						className="counter-button increment-1"
					>
						+1
					</button>
					<button
						onClick={() => increment(5)}
						disabled={!isConnected}
						className="counter-button increment-5"
					>
						+5
					</button>
					<button
						onClick={() => increment(10)}
						disabled={!isConnected}
						className="counter-button increment-10"
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
