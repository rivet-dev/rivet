const endpoint = "https://api.rivet.dev";
const token = process.env.RIVET_TOKEN ?? "";

const response = await fetch(`${endpoint}/namespaces`, {
	headers: { Authorization: `Bearer ${token}` },
});
const { namespaces, pagination } = await response.json();
console.log(namespaces, pagination.cursor);

export {};
