const endpoint = "https://api.rivet.dev";
const response = await fetch(
	`${endpoint}/gateway/${process.env.ACTOR_ID}/inspector/state`,
	{
		method: "PATCH",
		headers: {
			Authorization: `Bearer ${process.env.INSPECTOR_TOKEN}`,
			"Content-Type": "application/json",
		},
		body: JSON.stringify({ state: { count: 2 } }),
	},
);
console.log(await response.json());
export {};
