const http = require("node:http");
const fs = require("node:fs");
const path = require("node:path");

const html = fs.readFileSync(path.join(__dirname, "index.html"));
const port = Number(process.env.PORT) || 8080;

console.log("Hello from Rivet Compute! Waiting for you to deploy an image. See rivet.dev/docs/connect/rivet-compute to learn more");

const server = http.createServer((req, res) => {
	if (req.url === "/" || req.url === "/index.html") {
		res.writeHead(200, {
			"content-type": "text/html; charset=utf-8",
			"content-length": html.length,
			"cache-control": "no-store",
		});
		res.end(html);
		return;
	}
	res.writeHead(404, { "content-type": "text/plain; charset=utf-8" });
	res.end("not found");
});

server.listen(port, "0.0.0.0", () => {});
