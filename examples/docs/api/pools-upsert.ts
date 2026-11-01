const endpoint = "https://api.rivet.dev";
const namespace = process.env.RIVET_NAMESPACE ?? "default";
const token = process.env.RIVET_TOKEN ?? "";
const pool = process.env.RIVET_POOL ?? "default";

const params = new URLSearchParams({ namespace });
const response = await fetch(
	`${endpoint}/pools/${encodeURIComponent(pool)}?${params}`,
	{
		method: "PUT",
		headers: {
			Authorization: `Bearer ${token}`,
			"Content-Type": "application/json",
		},
		body: JSON.stringify({ drain_on_version_upgrade: true }),
	},
);
console.log(await response.json());

export {};
