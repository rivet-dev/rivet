const endpoint = "https://api.rivet.dev";
const response = await fetch(
	`${endpoint}/gateway/${process.env.ACTOR_ID}/inspector/queue`,
	{
		method: "POST",
		headers: {
			Authorization: `Bearer ${process.env.INSPECTOR_TOKEN}`,
			"Content-Type": "application/json",
		},
		body: JSON.stringify({ name: "increment", body: { amount: 1 } }),
	},
);
console.log(await response.json());
export {};
