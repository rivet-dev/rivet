const endpoint = "https://api.rivet.dev";
const namespace = process.env.RIVET_NAMESPACE ?? "default";
const token = process.env.RIVET_TOKEN ?? "";
const pool = process.env.RIVET_POOL ?? "default";

const params = new URLSearchParams({ namespace });
const response = await fetch(
	`${endpoint}/pools/${encodeURIComponent(pool)}?${params}`,
	{
		method: "DELETE",
		headers: { Authorization: `Bearer ${token}` },
	},
);
console.log(response.status);

export {};
