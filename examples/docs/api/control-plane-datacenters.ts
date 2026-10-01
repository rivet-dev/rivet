const endpoint = "https://api.rivet.dev";
const token = process.env.RIVET_TOKEN ?? "";

const response = await fetch(`${endpoint}/datacenters`, {
	headers: { Authorization: `Bearer ${token}` },
});
const { datacenters } = await response.json();
console.log(datacenters);

export {};
