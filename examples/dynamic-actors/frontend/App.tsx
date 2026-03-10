import { useEffect, useState } from "react";

const SOURCE_TEMPLATE = `import { actor } from "rivetkit";

export default actor({
	state: {
		count: 0,
	},
	actions: {
		increment: (c, amount = 1) => {
			c.state.count += amount;
			return c.state.count;
		},
		getCount: (c) => c.state.count,
	},
});
`;

type SourceResponse = {
	source: string;
	revision: number;
};

async function requestJson<T>(
	input: string,
	init?: RequestInit,
): Promise<T> {
	const response = await fetch(input, init);
	if (!response.ok) {
		const body = await response.text();
		throw new Error(`${response.status} ${response.statusText}: ${body}`);
	}
	return (await response.json()) as T;
}

function App() {
	const [source, setSource] = useState(SOURCE_TEMPLATE);
	const [revision, setRevision] = useState(1);
	const [dynamicKey, setDynamicKey] = useState("dynamic-main");
	const [count, setCount] = useState<number | null>(null);
	const [amount, setAmount] = useState(1);
	const [status, setStatus] = useState("Loading source actor...");
	const [loading, setLoading] = useState(false);

	useEffect(() => {
		void loadSource();
	}, []);

	const loadSource = async () => {
		setLoading(true);
		setStatus("Loading source...");
		try {
			const state = await requestJson<SourceResponse>("/api/source");
			setSource(state.source);
			setRevision(state.revision);
			setStatus(`Loaded source at revision ${state.revision}.`);
		} catch (error) {
			setStatus(`Failed to load source: ${String(error)}`);
		} finally {
			setLoading(false);
		}
	};

	const saveSource = async () => {
		setLoading(true);
		setStatus("Saving source...");
		try {
			const result = await requestJson<{ revision: number }>("/api/source", {
				method: "POST",
				headers: {
					"content-type": "application/json",
				},
				body: JSON.stringify({ source }),
			});
			setRevision(result.revision);
			setCount(null);
			setStatus(
				`Saved source at revision ${result.revision}. Generate a new dynamic key to force a fresh actor load.`,
			);
		} catch (error) {
			setStatus(`Failed to save source: ${String(error)}`);
		} finally {
			setLoading(false);
		}
	};

	const generateRandomKey = () => {
		const random = Math.random().toString(36).slice(2, 10);
		const nextKey = `dynamic-${random}`;
		setDynamicKey(nextKey);
		setCount(null);
		setStatus(`Switched to dynamic key "${nextKey}".`);
	};

	const getCount = async () => {
		const normalizedKey = dynamicKey.trim();
		if (!normalizedKey) {
			setStatus("Dynamic key cannot be empty.");
			return;
		}

		setLoading(true);
		setStatus("Running getCount() on dynamic actor...");
		try {
			const result = await requestJson<{ count: number }>(
				`/api/dynamic/${encodeURIComponent(normalizedKey)}/count`,
			);
			setCount(result.count);
			setStatus(`Dynamic actor count is ${result.count} (key "${normalizedKey}").`);
		} catch (error) {
			setStatus(`Dynamic actor call failed: ${String(error)}`);
		} finally {
			setLoading(false);
		}
	};

	const increment = async () => {
		const normalizedKey = dynamicKey.trim();
		if (!normalizedKey) {
			setStatus("Dynamic key cannot be empty.");
			return;
		}

		setLoading(true);
		setStatus("Running increment() on dynamic actor...");
		try {
			const result = await requestJson<{ count: number }>(
				`/api/dynamic/${encodeURIComponent(normalizedKey)}/increment`,
				{
					method: "POST",
					headers: {
						"content-type": "application/json",
					},
					body: JSON.stringify({ amount }),
				},
				);
				setCount(result.count);
				setStatus(
					`Incremented by ${amount}. Dynamic actor count is ${result.count} (key "${normalizedKey}").`,
				);
			} catch (error) {
				setStatus(`Dynamic actor call failed: ${String(error)}`);
			} finally {
				setLoading(false);
		}
	};

	return (
		<div style={{ maxWidth: 960, margin: "0 auto", padding: "2rem" }}>
			<h1>Dynamic Actor Editor</h1>
			<p>
				This example has two actors: <code>sourceCode</code> stores your source,
				 and <code>dynamicWorkflow</code> loads and runs that source.
			</p>

			<div style={{ display: "flex", gap: "0.75rem", marginBottom: "0.75rem" }}>
				<button onClick={loadSource} disabled={loading}>Reload Source</button>
				<button onClick={saveSource} disabled={loading}>Save Source</button>
				<button
					onClick={() => {
						setSource(SOURCE_TEMPLATE);
						setStatus("Template restored in editor. Click Save Source to persist.");
					}}
					disabled={loading}
				>
					Reset Template
				</button>
			</div>

			<textarea
				value={source}
				onChange={(event) => setSource(event.target.value)}
				spellCheck={false}
				style={{
					width: "100%",
					height: 360,
					fontFamily: "ui-monospace, SFMono-Regular, Menlo, monospace",
					fontSize: 14,
					lineHeight: 1.45,
					padding: "0.75rem",
					boxSizing: "border-box",
				}}
			/>

			<div style={{ marginTop: "1rem", display: "flex", gap: "0.75rem", alignItems: "center" }}>
				<label>
					Dynamic key:
					<input
						type="text"
						value={dynamicKey}
						onChange={(event) => setDynamicKey(event.target.value)}
						style={{ marginLeft: "0.5rem", width: 220 }}
					/>
				</label>
				<button onClick={generateRandomKey} disabled={loading}>
					Generate Random Key
				</button>
			</div>

			<div style={{ marginTop: "0.75rem", display: "flex", gap: "0.75rem", alignItems: "center" }}>
				<label>
					Amount:
					<input
						type="number"
						value={amount}
						onChange={(event) => setAmount(Number(event.target.value) || 0)}
						style={{ marginLeft: "0.5rem", width: 80 }}
					/>
				</label>
				<button onClick={getCount} disabled={loading}>getCount()</button>
				<button onClick={increment} disabled={loading}>increment(amount)</button>
			</div>

			<div style={{ marginTop: "1rem" }}>
				<div>
					<strong>Active revision:</strong> {revision}
				</div>
				<div>
					<strong>Active dynamic key:</strong> {dynamicKey}
				</div>
				<div>
					<strong>Last dynamic count:</strong>{" "}
					{count === null ? "not loaded" : count}
				</div>
				<div style={{ marginTop: "0.5rem" }}>
					<strong>Status:</strong> {status}
				</div>
			</div>
		</div>
	);
}

export default App;
