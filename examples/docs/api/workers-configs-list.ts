const endpoint = "https://api.rivet.dev";
const namespace = process.env.RIVET_NAMESPACE ?? "default";
const token = process.env.RIVET_TOKEN ?? "";

const params = new URLSearchParams({ namespace });
const response = await fetch(`${endpoint}/runner-configs?${params}`, {
	headers: { Authorization: `Bearer ${token}` },
});
const { runner_configs } = await response.json();
console.log(runner_configs);

export {};
