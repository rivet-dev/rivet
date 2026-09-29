const endpoint = "https://api.rivet.dev";
const query = new URLSearchParams({
	table: "users",
	limit: "100",
	offset: "0",
});
const response = await fetch(
	`${endpoint}/gateway/${process.env.ACTOR_ID}/inspector/database/rows?${query}`,
	{ headers: { Authorization: `Bearer ${process.env.INSPECTOR_TOKEN}` } },
);
console.log(await response.json());
export {};
