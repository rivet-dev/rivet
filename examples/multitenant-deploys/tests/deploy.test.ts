import { describe, it, expect } from "vitest";
import { app } from "../src/app";
import * as fs from "node:fs/promises";
import * as path from "node:path";
import * as os from "node:os";

describe("Deploy Endpoint", () => {
	it("should deploy an application and return endpoint", async () => {
		// Create a temporary Dockerfile for testing
		const tempDir = await fs.mkdtemp(path.join(os.tmpdir(), "rivet-test-"));
		const dockerfilePath = path.join(tempDir, "Dockerfile");

		await fs.writeFile(
			dockerfilePath,
			`
FROM node:22-alpine
WORKDIR /app
COPY . .

# Set env var from build arg
ARG MY_ENV_VAR
ENV MY_ENV_VAR=$MY_ENV_VAR
ARG APP_ID
ENV APP_ID=$APP_ID

# Create a non-root user
RUN addgroup -S rivetgroup && adduser -S rivet -G rivetgroup
USER rivet

CMD ["node", "server.js"]
    `,
		);

		// Create a simple server.js file
		const serverPath = path.join(tempDir, "server.js");
		await fs.writeFile(
			serverPath,
			`
const http = require("http");
const server = http.createServer((req, res) => {
  res.writeHead(200, { "Content-Type": "text/plain" });
  res.end(\`Hello \${process.env.MY_ENV_VAR} from \${process.env.APP_ID}!\`);
});
// Port defaults to 8080
console.log("Server started");
server.listen(8080);
    `,
		);

		// Create a FormData instance
		const formData = new FormData();

		// Generate a unique app ID for testing
		const testAppId = `test-${Math.floor(Math.random() * 10_000)}`;

		// Add the Dockerfile to the form data
		const dockerfileContent = await fs.readFile(dockerfilePath);
		const dockerfileBlob = new Blob([dockerfileContent], {
			type: "application/octet-stream",
		});
		formData.append("Dockerfile", dockerfileBlob, "Dockerfile");

		// Add server.js to the form data
		const serverContent = await fs.readFile(serverPath);
		const serverBlob = new Blob([serverContent], {
			type: "application/octet-stream",
		});
		formData.append("server.js", serverBlob, "server.js");

		// Make the request to the deploy endpoint
		const res = await app.request(
			`/deploy/${encodeURIComponent(testAppId)}`,
			{
				method: "POST",
				body: formData,
			},
		);

		// Verify the response
		expect(res.status).toBe(200);

		const responseData = await res.json();
		expect(responseData.success).toBe(true);
		expect(responseData.appId).toBe(testAppId);
		expect(responseData.endpoint).toBeDefined();

		// Clean up the temporary directory
		await fs.rm(tempDir, { recursive: true, force: true });
	}, 120000); // 2 minute timeout for this test
});
