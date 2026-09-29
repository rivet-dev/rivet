const endpoint = "https://api.rivet.dev";
const actorId = process.env.ACTOR_ID;
const response = await fetch(`${endpoint}/gateway/${actorId}/queue/increment`, {
	method: "POST",
	headers: {
		Authorization: `Bearer ${process.env.RIVET_TOKEN}`,
		"Content-Type": "application/json",
	},
	body: JSON.stringify({ body: { amount: 1 }, wait: true, timeout: 5000 }),
});
console.log(await response.json());
export {};
