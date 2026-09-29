const endpoint = "https://api.rivet.dev";
const response = await fetch(
	`${endpoint}/gateway/${process.env.ACTOR_ID}/inspector/queue`,
	{
		method: "DELETE",
		headers: { Authorization: `Bearer ${process.env.INSPECTOR_TOKEN}` },
	},
);
console.log(await response.json());
export {};
