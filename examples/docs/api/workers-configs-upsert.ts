const endpoint = "https://api.rivet.dev";
const namespace = process.env.RIVET_NAMESPACE ?? "default";
const token = process.env.RIVET_TOKEN ?? "";
const workerName = process.env.RUNNER_NAME ?? "default";

const params = new URLSearchParams({ namespace });
const response = await fetch(
	`${endpoint}/runner-configs/${encodeURIComponent(workerName)}?${params}`,
	{
		method: "PUT",
		headers: {
			Authorization: `Bearer ${token}`,
			"Content-Type": "application/json",
		},
		body: JSON.stringify({ datacenters: { local: { normal: {} } } }),
	},
);
console.log(await response.json());

export {};
