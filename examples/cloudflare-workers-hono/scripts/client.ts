async function main() {
	const endpoint = process.env.RIVETKIT_ENDPOINT || "http://localhost:8787";
	const res = await fetch(`${endpoint}/increment/foo`, {
		method: "POST",
	});
	console.log("Output:", await res.text());
}

main();
