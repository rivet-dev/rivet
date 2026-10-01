const endpoint = "https://api.rivet.dev";

const response = await fetch(`${endpoint}/metadata`);
const metadata = await response.json();
console.log(metadata);

export {};
