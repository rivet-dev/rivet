import { createRivetKit } from "@rivetkit/react";
import { useEffect, useState } from "react";
import type { registry } from "../src/registry";

const { useActor } = createRivetKit<typeof registry>("http://localhost:8080");

export function App() {
	const [topValues, setTopValues] = useState<number[]>([]);
	const [newValue, setNewValue] = useState<number>(0);
	const [totalCount, setTotalCount] = useState<number>(0);
	const [highestValue, setHighestValue] = useState<number | null>(null);

	const streamProcessor = useActor({
		name: "streamProcessor",
		key: ["global"],
	});

	// Load initial stats
	useEffect(() => {
		if (streamProcessor.connection) {
			streamProcessor.connection.getStats().then((stats) => {
				setTopValues(stats.topValues);
				setTotalCount(stats.totalCount);
				setHighestValue(stats.highestValue);
			});
		}
	}, [streamProcessor.connection]);

	// Listen for updates from other clients
	streamProcessor.useEvent("updated", ({ topValues, totalCount, highestValue }: {
		topValues: number[];
		totalCount: number;
		highestValue: number | null;
	}) => {
		setTopValues(topValues);
		setTotalCount(totalCount);
		setHighestValue(highestValue);
	});

	// Add a new value to the stream
	const handleAddValue = async () => {
		if (streamProcessor.connection && !isNaN(newValue)) {
			await streamProcessor.connection.addValue(newValue);
			setNewValue(0);
		}
	};

	// Reset the stream
	const handleReset = async () => {
		if (streamProcessor.connection) {
			const result = await streamProcessor.connection.reset();
			setTopValues(result.topValues);
			setTotalCount(result.totalCount);
			setHighestValue(result.highestValue);
		}
	};

	// Handle form submission
	const handleSubmit = (e: React.FormEvent) => {
		e.preventDefault();
		handleAddValue();
	};

	// Handle random value generation
	const handleRandomValue = () => {
		const randomValue = Math.floor(Math.random() * 1000) + 1;
		setNewValue(randomValue);
	};

	return (
		<div className="app-container">
			<div className="header">
				<h1>Stream Processor</h1>
				<p>Real-time top-3 value tracking with RivetKit</p>
			</div>

			<div className="info-box">
				<h3>How it works</h3>
				<p>
					This stream processor maintains the top 3 highest values in real-time. 
					Add numbers and watch as the system automatically keeps track of the highest values. 
					All connected clients see updates immediately when new values are added.
				</p>
			</div>

			<div className="content">
				<div className="top-values-section">
					<div className="top-values-list">
						<h3>🏆 Top 3 Values</h3>
						{topValues.length === 0 ? (
							<div className="empty-state">
								No values added yet.<br />
								Add some numbers to get started!
							</div>
						) : (
							topValues.map((value, index) => (
								<div key={`${value}-${index}`} className="value-item">
									<span className="value-rank">#{index + 1}</span>
									<span className="value-number">{value.toLocaleString()}</span>
								</div>
							))
						)}
					</div>
				</div>

				<div className="input-section">
					<form onSubmit={handleSubmit} className="input-form">
						<h3>Add New Value</h3>
						
						<div className="input-group">
							<label htmlFor="value-input">Number:</label>
							<input
								id="value-input"
								type="number"
								value={newValue || ""}
								onChange={(e) => setNewValue(Number(e.target.value))}
								placeholder="Enter any number..."
								disabled={!streamProcessor.connection}
							/>
						</div>

						<button 
							type="submit" 
							className="submit-button"
							disabled={!streamProcessor.connection || isNaN(newValue)}
						>
							Add to Stream
						</button>
					</form>

					<div style={{ marginTop: "15px", display: "flex", gap: "10px" }}>
						<button 
							onClick={handleRandomValue}
							style={{
								flex: 1,
								padding: "8px",
								backgroundColor: "#28a745",
								color: "white",
								border: "none",
								borderRadius: "4px",
								cursor: "pointer"
							}}
						>
							Random Value
						</button>
						<button 
							onClick={handleReset}
							disabled={!streamProcessor.connection}
							style={{
								flex: 1,
								padding: "8px",
								backgroundColor: "#dc3545",
								color: "white",
								border: "none",
								borderRadius: "4px",
								cursor: "pointer"
							}}
						>
							Reset Stream
						</button>
					</div>
				</div>
			</div>

			<div className="stats">
				<div className="stat-item">
					<div className="stat-value">{totalCount}</div>
					<div className="stat-label">Total Values</div>
				</div>
				<div className="stat-item">
					<div className="stat-value">{highestValue?.toLocaleString() || "—"}</div>
					<div className="stat-label">Highest Value</div>
				</div>
				<div className="stat-item">
					<div className="stat-value">{topValues.length}</div>
					<div className="stat-label">Top Values Count</div>
				</div>
			</div>
		</div>
	);
}
