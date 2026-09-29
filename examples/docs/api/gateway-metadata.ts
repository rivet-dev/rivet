const endpoint = "https://api.rivet.dev";
const response = await fetch(
	`${endpoint}/gateway/${process.env.ACTOR_ID}/metadata`,
	{
		headers: { Authorization: `Bearer ${process.env.RIVET_TOKEN}` },
	},
);
console.log(await response.json());
export {};
