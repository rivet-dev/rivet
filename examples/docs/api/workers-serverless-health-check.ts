const endpoint = "https://api.rivet.dev";
const namespace = process.env.RIVET_NAMESPACE ?? "default";
const token = process.env.RIVET_TOKEN ?? "";

const params = new URLSearchParams({ namespace });
const response = await fetch(
	`${endpoint}/runner-configs/serverless-health-check?${params}`,
	{
		method: "POST",
		headers: {
			Authorization: `Bearer ${token}`,
			"Content-Type": "application/json",
		},
		body: JSON.stringify({
			url: "https://workers.example.com",
			headers: {},
		}),
	},
);
console.log(await response.json());

export {};
