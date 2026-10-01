const endpoint = "https://api.rivet.dev";
const token = process.env.RIVET_TOKEN ?? "";

// Inspect reports on the token used to authenticate the request itself.
const response = await fetch(`${endpoint}/auth/tokens/inspect`, {
	headers: { Authorization: `Bearer ${token}` },
});

const { namespace_id, subject, grants, expires_ts } = await response.json();
console.log(namespace_id, subject, grants, new Date(expires_ts));

export {};
