const endpoint = "https://api.rivet.dev";
const response = await fetch(
	`${endpoint}/gateway/${process.env.ACTOR_ID}/inspector/database/execute`,
	{
		method: "POST",
		headers: {
			Authorization: `Bearer ${process.env.INSPECTOR_TOKEN}`,
			"Content-Type": "application/json",
		},
		body: JSON.stringify({
			sql: "SELECT * FROM users WHERE id = ?",
			args: ["user-123"],
		}),
	},
);
console.log(await response.json());
export {};
