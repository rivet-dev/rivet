import { createRivetKit } from "@rivetkit/react";
import { useEffect, useState } from "react";
import type { CountChangedEvent, registry } from "../src/actors.ts";

const { useActor } = createRivetKit<typeof registry>(`${location.origin}/api/rivet`);

function LightbulbIcon() {
	return (
		<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
			<path d="M9 18h6" />
			<path d="M10 22h4" />
			<path d="M15.09 14c.18-.98.65-1.74 1.41-2.5A4.65 4.65 0 0 0 18 8 6 6 0 0 0 6 8c0 1 .23 2.23 1.5 3.5A4.61 4.61 0 0 1 8.91 14" />
		</svg>
	);
}

function ArrowRightIcon() {
	return (
		<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
			<path d="M5 12h14" />
			<path d="m12 5 7 7-7 7" />
		</svg>
	);
}

function formatTimeAgo(timestamp: number): string {
	if (timestamp === 0) return "never";
	const seconds = Math.floor((Date.now() - timestamp) / 1000);
	if (seconds < 5) return "just now";
	if (seconds < 60) return `${seconds}s ago`;
	const minutes = Math.floor(seconds / 60);
	if (minutes < 60) return `${minutes}m ago`;
	const hours = Math.floor(minutes / 60);
	return `${hours}h ago`;
}

export function App() {
	const [actorKey, setActorKey] = useState("default");
	const [count, setCount] = useState<number>(0);
	const [lastUpdatedAt, setLastUpdatedAt] = useState<number>(0);
	const [isAnimating, setIsAnimating] = useState(false);
	const [, forceUpdate] = useState(0);

	const counter = useActor({
		name: "counter",
		key: [actorKey],
	});

	useEffect(() => {
		if (counter.connection) {
			counter.connection.getState().then((state) => {
				setCount(state.count);
				setLastUpdatedAt(state.lastUpdatedAt);
			});
		}
	}, [counter.connection]);

	// Update "time ago" display periodically
	useEffect(() => {
		const interval = setInterval(() => {
			forceUpdate((n) => n + 1);
		}, 1000);
		return () => clearInterval(interval);
	}, []);

	counter.useEvent("countChanged", (event: CountChangedEvent) => {
		setCount(event.count);
		setLastUpdatedAt(event.updatedAt);

		// Trigger animation
		setIsAnimating(true);
		setTimeout(() => setIsAnimating(false), 300);
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

	return (
		<div className="counter-app">
			<div className="counter-container">
				<div className="counter-header">
					<div className="header-title">
						<h1>Rivet Actors Demo</h1>
						<span className="header-tagline">Build stateful backends in minutes</span>
					</div>
					<div className={`status-indicator ${counter.connection ? 'connected' : 'disconnected'}`}>
						<div className="status-dot"></div>
						<span>{counter.connection ? 'Connected' : 'Connecting...'}</span>
					</div>
				</div>

				<div className="counter-settings">
					<div className="setting-group">
						<label htmlFor="actorKey">Actor Key</label>
						<input
							id="actorKey"
							type="text"
							value={actorKey}
							onChange={(e) => setActorKey(e.target.value)}
							placeholder="Enter actor key"
							className="setting-input"
						/>
					</div>
				</div>

				<div className="counter-display">
					<div className={`count-value ${isAnimating ? 'changed' : ''}`}>
						{count}
					</div>
					<p className="count-label">Current Count</p>
					<p className="last-updated">
						Last updated: {formatTimeAgo(lastUpdatedAt)}
					</p>
				</div>

				<div className="counter-controls">
					<button
						onClick={() => decrement(5)}
						disabled={!counter.connection}
						className="counter-button"
					>
						-5
					</button>
					<button
						onClick={() => decrement(1)}
						disabled={!counter.connection}
						className="counter-button"
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
				</div>

				<div className="reset-section">
					<button
						onClick={reset}
						disabled={!counter.connection}
						className="reset-button"
					>
						Reset Counter
					</button>
				</div>

				<div className="info-box">
					<div className="info-box-header">
						<LightbulbIcon />
						<span>What's happening?</span>
					</div>
					<p>
						This counter is powered by a Rivet Actor — a persistent, stateful process that:
					</p>
					<ul>
						<li>Survives restarts and deployments</li>
						<li>Syncs instantly across all clients</li>
						<li>Scales automatically with your users</li>
					</ul>
					<p>
						Try opening this page in multiple tabs to see real-time sync in action!
					</p>
					<div className="info-box-links">
						<a
							href="https://rivet.dev/docs"
							target="_blank"
							rel="noopener noreferrer"
							className="info-box-link"
						>
							Read the Docs
							<ArrowRightIcon />
						</a>
					</div>
				</div>
			</div>
		</div>
	);
}
