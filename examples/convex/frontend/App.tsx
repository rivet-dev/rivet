import { useState, useEffect } from "react";
import { createRivetKit } from "@rivetkit/react";
import type { registry } from "../convex/rivet/actors";

// Derive Convex site URL from cloud URL (HTTP routes use .convex.site, not .convex.cloud)
const CONVEX_URL = import.meta.env.VITE_CONVEX_URL || "http://localhost:3000";
const CONVEX_SITE_URL = CONVEX_URL.replace(".convex.cloud", ".convex.site");

const { useActor } = createRivetKit<typeof registry>(
	`${CONVEX_SITE_URL}/api/rivet`,
);

export function App() {
	const { connection, useEvent } = useActor({ name: "counter", key: ["main"] });

	const [count, setCount] = useState(0);

	useEvent("newCount", (newCount: number) => {
		setCount(newCount);
	});

	useEffect(() => {
		if (connection) {
			connection.getCount().then(setCount);
		}
	}, [connection]);

	return (
		<div className="container">
			<h1>RivetKit + Convex Counter</h1>
			<p className="count">Count: {count}</p>
			<button
				onClick={() => connection?.increment(1)}
				disabled={!connection}
			>
				Increment
			</button>
		</div>
	);
}
