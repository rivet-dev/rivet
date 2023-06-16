addEventListener("fetch", (event) => {
	event.respondWith(handleRequest(event.request));
});

async function handleRequest(request) {
	let { timezone, asn, latitude, longitude } = request.cf;

	let originRequest = new Request(request);
	originRequest.headers.set("X-Timezone", timezone);
	originRequest.headers.set("X-Coords", latitude + "," + longitude);
	originRequest.headers.set("X-ASN", asn);

	return await fetch(originRequest);
}

