const endpoint = "https://api.rivet.dev";
const namespace = process.env.RIVET_NAMESPACE ?? "default";
const token = process.env.RIVET_TOKEN ?? "";
const workerName = process.env.RUNNER_NAME ?? "default";

const params = new URLSearchParams({ namespace });
const response = await fetch(
	`${endpoint}/runner-configs/${encodeURIComponent(workerName)}?${params}`,
	{ method: "DELETE", headers: { Authorization: `Bearer ${token}` } },
);
console.log(response.ok);

export {};
