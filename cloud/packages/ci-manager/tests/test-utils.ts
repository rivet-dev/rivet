import { mkdir, writeFile, readFile, unlink } from "node:fs/promises";
import { join } from "node:path";
import { execSync } from "node:child_process";
import * as tar from "tar";
import { randomUUID } from "crypto";

export async function createFailingDockerContext(): Promise<Buffer> {
	const tempDir = `/tmp/docker-context-fail-${Date.now()}-${Math.random().toString(36).substring(2)}`;
	await mkdir(tempDir, { recursive: true });
	await mkdir(join(tempDir, "configs"), { recursive: true });

	const dockerfile = `
FROM alpine:latest
RUN exit 1
CMD echo "This should never run"
`;

	await writeFile(join(tempDir, "configs/fail.dockerfile"), dockerfile);

	const tarPath = `/tmp/context-fail-${Date.now()}.tar.gz`;

	await tar.create(
		{
			gzip: true,
			file: tarPath,
			cwd: tempDir,
		},
		["."],
	);

	const fileBuffer = await readFile(tarPath);
	return fileBuffer;
}

export async function createTestWebServerImage(): Promise<{
	dockerTarPath: string;
	imageName: string;
}> {
	const testId = randomUUID();
	const imageName = `rivet-build:${testId}`;
	const contextDir = `/tmp/webserver-context-${testId}`;
	await mkdir(contextDir, { recursive: true });

	const dockerfile = `
FROM node:18-alpine
WORKDIR /app
COPY package.json server.js ./
RUN npm install
EXPOSE 3000
CMD ["node", "server.js"]
`;

	const packageJson = {
		name: "test-webserver",
		version: "1.0.0",
		dependencies: {
			express: "^4.18.0",
		},
	};

	const serverJs = `
const express = require('express');
const app = express();
const port = 3000;

app.get('/', (req, res) => {
	res.json({ 
		message: 'Rivet test successful!',
		timestamp: new Date().toISOString(),
		hostname: require('os').hostname()
	});
});

app.get('/health', (req, res) => {
	res.json({ status: 'healthy' });
});

app.listen(port, '0.0.0.0', () => {
	console.log(\`Test server listening on port \${port}\`);
});
`;

	await mkdir(join(contextDir, "docker"), { recursive: true });
	await writeFile(
		join(contextDir, "docker/app.dockerfile"),
		dockerfile.trim(),
	);
	await writeFile(
		join(contextDir, "package.json"),
		JSON.stringify(packageJson, null, 2),
	);
	await writeFile(join(contextDir, "server.js"), serverJs.trim());

	console.log(`Building test web server image: ${imageName}`);
	execSync(`docker build -f docker/app.dockerfile -t ${imageName} .`, {
		cwd: contextDir,
		stdio: "pipe",
	});

	const dockerTarPath = `/tmp/webserver-image-${testId}.tar`;
	console.log(`Saving Docker image to: ${dockerTarPath}`);
	execSync(`docker save -o ${dockerTarPath} ${imageName}`, {
		stdio: "pipe",
	});

	return { dockerTarPath, imageName };
}

export async function createTestWebServerContext(): Promise<Buffer> {
	const testId = randomUUID();
	const tempDir = `/tmp/webserver-context-${testId}`;
	await mkdir(tempDir, { recursive: true });
	await mkdir(join(tempDir, "configs"), { recursive: true });

	const dockerfile = `
FROM node:18-alpine
WORKDIR /app
COPY package.json server.js ./
RUN npm install
EXPOSE 3000
CMD ["node", "server.js"]
`;

	const packageJson = {
		name: "test-webserver",
		version: "1.0.0",
		dependencies: {
			express: "^4.18.0",
		},
	};

	const serverJs = `
const express = require('express');
const app = express();
const port = 3000;

app.get('/', (req, res) => {
	res.json({ 
		message: 'Rivet test successful!',
		timestamp: new Date().toISOString(),
		hostname: require('os').hostname()
	});
});

app.get('/health', (req, res) => {
	res.json({ status: 'healthy' });
});

app.listen(port, '0.0.0.0', () => {
	console.log(\`Test server listening on port \${port}\`);
});
`;

	await writeFile(
		join(tempDir, "configs/build.dockerfile"),
		dockerfile.trim(),
	);
	await writeFile(
		join(tempDir, "package.json"),
		JSON.stringify(packageJson, null, 2),
	);
	await writeFile(join(tempDir, "server.js"), serverJs.trim());

	const tarPath = `/tmp/webserver-context-${testId}.tar.gz`;

	await tar.create(
		{
			gzip: true,
			file: tarPath,
			cwd: tempDir,
		},
		["."],
	);

	const fileBuffer = await readFile(tarPath);
	return fileBuffer;
}

export async function downloadOutputTar(
	serverUrl: string,
	buildId: string,
): Promise<Buffer> {
	const response = await fetch(
		`${serverUrl}/builds/${buildId}/output.tar.gz`,
	);

	if (!response.ok) {
		throw new Error(`Failed to download output: ${response.statusText}`);
	}

	const arrayBuffer = await response.arrayBuffer();
	return Buffer.from(arrayBuffer);
}

export async function pollBuildStatus(
	baseUrl: string,
	buildId?: string,
): Promise<{ status: string; buildId: string }> {
	const maxAttempts = 150;
	const interval = 2000;

	for (let attempt = 0; attempt < maxAttempts; attempt++) {
		try {
			const response = await fetch(`${baseUrl}/builds/${buildId}`);
			if (!response.ok) {
				throw new Error(`HTTP ${response.status}`);
			}

			const build = await response.json();
			console.log(
				`[${buildId}] Status: ${build.status.type} ${JSON.stringify(build.status.data)}`,
			);

			if (
				build.status.type === "success" ||
				build.status.type === "failure"
			) {
				return { status: build.status.type, buildId: build.status.data?.buildId };
			}

			if (attempt < maxAttempts - 1) {
				await new Promise((resolve) => setTimeout(resolve, interval));
			}
		} catch (error) {
			console.log(
				`[${buildId}] Poll attempt ${attempt + 1} failed:`,
				error,
			);
			if (attempt < maxAttempts - 1) {
				await new Promise((resolve) => setTimeout(resolve, interval));
			}
		}
	}

	throw new Error(`Build polling timeout after ${maxAttempts} attempts`);
}

export async function createBuildWithContext(
	baseUrl: string,
	buildName: string,
	dockerfilePath: string,
	contextBuffer: Buffer,
	environmentId?: string,
): Promise<string> {
	const formData = new FormData();
	formData.append("buildName", buildName);
	formData.append("dockerfilePath", dockerfilePath);
	if (environmentId) {
		formData.append("environmentId", environmentId);
	}
	formData.append(
		"context",
		new Blob([contextBuffer], { type: "application/gzip" }),
	);

	const response = await fetch(`${baseUrl}/builds`, {
		method: "POST",
		body: formData,
	});

	if (!response.ok) {
		throw new Error(`Failed to create build: ${response.statusText}`);
	}

	const result = await response.json();
	return result.buildId;
}

export async function getBuildStatus(
	baseUrl: string,
	buildId: string,
): Promise<any> {
	const response = await fetch(`${baseUrl}/builds/${buildId}`);
	return await response.json();
}


export async function testActorEndpoint(endpoint: string): Promise<any> {
	const response = await fetch(`${endpoint}`, {
		method: "GET",
		headers: { "User-Agent": "rivet-test" },
	});

	if (!response.ok) {
		throw new Error(`HTTP ${response.status}: ${response.statusText}`);
	}

	return await response.json();
}

export async function createActorFromBuild(
	buildId: string,
	buildName: string,
	rivetConfig: { token: string; project: string; environment: string },
): Promise<{ actorId: string; endpoint: string }> {
	const { RivetClient } = await import("@rivet-gg/api");
	const client = new RivetClient({ token: rivetConfig.token });

	console.log(`Creating actor with build ${buildName} and ID ${buildId}...`);
	const { actor } = await client.actors.create({
		project: rivetConfig.project,
		environment: rivetConfig.environment,
		body: {
			tags: { name: buildName },
			build: buildId,
			network: {
				ports: {
					http: {
						protocol: "https",
						internalPort: 3000,
					},
				},
			},
			resources: {
				cpu: 100,
				memory: 128,
			},
		},
	});

	const endpoint = actor.network?.ports?.http?.url;
	if (!endpoint) {
		throw new Error("Actor endpoint not available");
	}

	return {
		actorId: actor.id,
		endpoint,
	};
}

export async function waitForActorReady(endpoint: string): Promise<void> {
	const maxAttempts = 30;
	const interval = 2000;

	for (let attempt = 0; attempt < maxAttempts; attempt++) {
		try {
			const response = await fetch(`${endpoint}/health`, {
				method: "GET",
				headers: { "User-Agent": "rivet-test" },
			});

			if (response.ok) {
				console.log("Actor is ready!");
				return;
			}
		} catch (error) {
			console.log(`Waiting for actor to be ready... (attempt ${attempt + 1})`);
		}

		if (attempt < maxAttempts - 1) {
			await new Promise((resolve) => setTimeout(resolve, interval));
		}
	}

	throw new Error(`Actor not ready after ${maxAttempts} attempts`);
}

