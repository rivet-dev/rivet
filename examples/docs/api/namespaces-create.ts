const endpoint = "https://api.rivet.dev";
const token = process.env.RIVET_TOKEN ?? "";

const response = await fetch(`${endpoint}/namespaces`, {
	method: "POST",
	headers: {
		Authorization: `Bearer ${token}`,
		"Content-Type": "application/json",
	},
	body: JSON.stringify({ name: "my-app", display_name: "My App" }),
});
const { namespace } = await response.json();
console.log(namespace);

export {};
