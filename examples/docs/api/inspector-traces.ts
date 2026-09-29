const endpoint = "https://api.rivet.dev";
const response = await fetch(
	`${endpoint}/gateway/${process.env.ACTOR_ID}/inspector/traces?startMs=0&limit=100`,
	{ headers: { Authorization: `Bearer ${process.env.INSPECTOR_TOKEN}` } },
);
console.log(await response.json());
export {};
