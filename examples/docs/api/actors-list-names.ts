const endpoint = "https://api.rivet.dev";
const namespace = process.env.RIVET_NAMESPACE ?? "default";
const token = process.env.RIVET_TOKEN ?? "";

const response = await fetch(
	`${endpoint}/actors/names?namespace=${namespace}`,
	{
		headers: { Authorization: `Bearer ${token}` },
	},
);

const { names } = await response.json();
console.log(Object.keys(names)); // ["counter", ...]

export {};
