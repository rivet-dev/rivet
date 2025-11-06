import { createRivetKit } from "@rivetkit/react";
import { useEffect, useState } from "react";
import type { RateLimitResult, registry } from "../backend/registry";

const { useActor } = createRivetKit<typeof registry>("http://localhost:6420");

function RateLimiterDemo({ userId }: { userId: string }) {
	const [result, setResult] = useState<RateLimitResult | null>(null);
	const [loading, setLoading] = useState(false);

	const rateLimiter = useActor({
		name: "rateLimiter",
		key: [userId],
	});

	useEffect(() => {
		if (rateLimiter.connection) {
			// Get initial status
			rateLimiter.connection.getStatus().then((status) => {
				setResult({
					allowed: status.remaining > 0,
					remaining: status.remaining,
					resetsIn: status.resetsIn,
				});
			});
		}
	}, [rateLimiter.connection]);

	const makeRequest = async () => {
		if (!rateLimiter.connection || loading) return;

		setLoading(true);
		try {
			const response = await rateLimiter.connection.checkLimit();
			setResult(response);
		} finally {
			setLoading(false);
		}
	};

	const resetLimiter = async () => {
		if (!rateLimiter.connection) return;

		await rateLimiter.connection.reset();
		// Get updated status
		const status = await rateLimiter.connection.getStatus();
		setResult({
			allowed: status.remaining > 0,
			remaining: status.remaining,
			resetsIn: status.resetsIn,
		});
	};

	const usagePercentage = result ? ((5 - result.remaining) / 5) * 100 : 0;

	return (
		<div className="rate-limiter-demo">
			<button
				className="request-button"
				onClick={makeRequest}
				disabled={!rateLimiter.connection || loading}
			>
				{loading ? "Making Request..." : "Make API Request"}
			</button>

			{result && (
				<div className="status-display">
					<div className="status-item">
						<span className="status-label">Status:</span>
						<span className={`status-value ${result.allowed ? 'allowed' : 'blocked'}`}>
							{result.allowed ? "✓ Request Allowed" : "✖ Request Blocked"}
						</span>
					</div>
					<div className="status-item">
						<span className="status-label">Remaining Requests:</span>
						<span className="status-value">{result.remaining} / 5</span>
					</div>
					<div className="status-item">
						<span className="status-label">Rate Limit Usage:</span>
						<div style={{ flex: 1, marginLeft: "20px" }}>
							<div className="progress-bar">
								<div 
									className="progress-fill" 
									style={{ width: `${usagePercentage}%` }}
								/>
							</div>
						</div>
					</div>
					<div className="status-item">
						<span className="status-label">Resets In:</span>
						<span className="status-value">{result.resetsIn} seconds</span>
					</div>
				</div>
			)}

			<button className="reset-button" onClick={resetLimiter}>
				Reset Rate Limiter (Admin)
			</button>
		</div>
	);
}

export function App() {
	const [selectedUser, setSelectedUser] = useState("user-1");

	const users = [
		{ id: "user-1", name: "User 1" },
		{ id: "user-2", name: "User 2" },
		{ id: "user-3", name: "User 3" },
		{ id: "api-client-1", name: "API Client 1" },
		{ id: "api-client-2", name: "API Client 2" },
	];

	return (
		<div className="app-container">
			<div className="header">
				<h1>Rate Limiter Demo</h1>
				<p>5 requests per minute per user/client</p>
			</div>

			<div className="content">
				<div className="info-box">
					<h3>How it works</h3>
					<p>
						This rate limiter allows 5 requests per minute per user. Each user gets their own 
						independent rate limit counter. When the limit is exceeded, further requests are 
						blocked until the window resets. Switch between users to see isolated rate limiting.
					</p>
				</div>

				<div className="user-selector">
					<label>Select User/Client:</label>
					<select
						value={selectedUser}
						onChange={(e) => setSelectedUser(e.target.value)}
					>
						{users.map((user) => (
							<option key={user.id} value={user.id}>
								{user.name}
							</option>
						))}
					</select>
				</div>

				<RateLimiterDemo key={selectedUser} userId={selectedUser} />
			</div>
		</div>
	);
}
