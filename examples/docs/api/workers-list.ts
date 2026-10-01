const endpoint = "https://api.rivet.dev";
const namespace = process.env.RIVET_NAMESPACE ?? "default";
const token = process.env.RIVET_TOKEN ?? "";

const params = new URLSearchParams({ namespace });
const response = await fetch(`${endpoint}/envoys?${params}`, {
	headers: { Authorization: `Bearer ${token}` },
});
const { envoys: workers, pagination } = await response.json();
console.log(workers, pagination.cursor);

export {};
