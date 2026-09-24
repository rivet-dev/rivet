import { useEffect, useState } from "react";
import { createClient, type ActorConn } from "rivetkit/client";
import type { counter as counterDefinition, registry } from "../src/actors.ts";

type Connection = {
	counter: ActorConn<typeof counterDefinition>;
	count: number;
};

export default function App() {
	const [connection, setConnection] = useState<Connection | null>(null);
	const [pending, setPending] = useState(false);
	const [error, setError] = useState("");
	const [tokenRequests, setTokenRequests] = useState(0);
	const actorConnection = connection?.counter;

	useEffect(() => {
		return () => {
			void actorConnection?.dispose();
		};
	}, [actorConnection]);

	async function connect() {
		setPending(true);
		setError("");
		setTokenRequests(0);
		let counter: ActorConn<typeof counterDefinition> | undefined;
		try {
			// 1. Find the demo counter. No login is needed in this example.
			const response = await fetch("/api/counter");
			if (!response.ok) throw new Error("Could not load the counter");
			const { endpoint, namespace, actorId } = await response.json();
			// 2. Let RivetKit request the first token and renew it through getToken.
			const client = createClient<typeof registry>({
				endpoint,
				namespace,
				getToken: async () => {
					try {
						const { token } = await requestToken();
						setTokenRequests((count) => count + 1);
						return token;
					} catch (error) {
						setConnection(null);
						setError(
							"Could not refresh access. Connect again to retry.",
						);
						throw error;
					}
				},
			});
			counter = client.counter.getForId(actorId).connect();
			setConnection({ counter, count: await counter.getCount() });
		} catch (error) {
			void counter?.dispose();
			setError(
				error instanceof Error ? error.message : "Could not connect",
			);
		} finally {
			setPending(false);
		}
	}

	async function increment() {
		if (!connection) return;
		setPending(true);
		setError("");
		try {
			const count = await connection.counter.increment(1);
			setConnection((current) =>
				current ? { ...current, count } : null,
			);
		} catch {
			setError("Could not update the counter. Please try again.");
		} finally {
			setPending(false);
		}
	}

	return (
		<main>
			<p className="eyebrow">RivetKit example</p>
			<h1>Scoped JWT counter</h1>
			<p className="intro">
				Get a token from the server, then connect to a counter actor.
			</p>
			{connection ? (
				<>
					<p className="status">Connected</p>
					<p className="count" aria-live="polite" aria-label="Count">
						{connection.count}
					</p>
					<button
						type="button"
						onClick={increment}
						disabled={pending}
					>
						{pending ? "Updating…" : "Increment"}
					</button>
					<p className="hint" aria-live="polite">
						Token lifetime: 30 seconds. Refreshes:{" "}
						{Math.max(0, tokenRequests - 1)}.
					</p>
					<p className="hint">
						Access renews automatically. The connection refreshes at
						about 60 seconds, including Engine’s 30-second grace
						period.
					</p>
				</>
			) : (
				<button type="button" onClick={connect} disabled={pending}>
					{pending ? "Connecting…" : "Connect to counter"}
				</button>
			)}
			{error && <p role="alert">{error}</p>}
		</main>
	);
}

async function requestToken() {
	const response = await fetch("/api/token", { method: "POST" });
	if (!response.ok) throw new Error("Could not get a token");
	return (await response.json()) as {
		endpoint: string;
		namespace: string;
		actorId: string;
		token: string;
	};
}
