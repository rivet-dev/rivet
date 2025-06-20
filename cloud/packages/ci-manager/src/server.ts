import { Hono } from "hono";
import { streamSSE } from "hono/streaming";
import { logger } from "hono/logger";
import { BuildStore } from "./build-store";
import { runKanikoBuild } from "./kaniko-runner";
import { createWriteStream, createReadStream } from "node:fs";
import { mkdir, stat } from "node:fs/promises";
import { dirname } from "path";
import { Readable } from "node:stream";
import { pipeline } from "node:stream/promises";
import { serve } from "@hono/node-server";
import type { ReadableStream as WebReadableStream } from "stream/web";
import {
	convertDockerTarToOCIBundle,
} from "./oci-converter";
import {
	uploadOCIBundleToRivet,
	type RivetUploadConfig,
} from "./rivet-uploader";

async function processRivetUpload(
	buildStore: BuildStore,
	buildId: string,
): Promise<void> {
	const build = buildStore.getBuild(buildId);
	if (!build || !build.outputPath) {
		throw new Error(`Build ${buildId} not found or missing output path`);
	}

	try {
		// Check if Rivet upload is enabled
		const rivetToken = process.env.RIVET_CLOUD_TOKEN;
		const rivetProject = process.env.RIVET_PROJECT;
		const rivetEnvironment = process.env.RIVET_ENVIRONMENT;

		if (!rivetToken || !rivetProject || !rivetEnvironment) {
			throw new Error(
				"Rivet upload failed - missing RIVET_CLOUD_TOKEN, RIVET_PROJECT, or RIVET_ENVIRONMENT",
			);
		}

		const rivetConfig: RivetUploadConfig = {
			token: rivetToken,
			project: rivetProject,
			environment: build.environmentId,
		};

		// Step 1: Convert to OCI bundle
		buildStore.updateStatus(buildId, { type: "converting", data: {} });
		buildStore.addLog(buildId, "Converting Docker image to OCI bundle...");

		const conversionResult = await convertDockerTarToOCIBundle(
			build.outputPath,
		);

		try {
			// Step 2: Upload to Rivet
			buildStore.updateStatus(buildId, {
				type: "uploading",
				data: {},
			});
			buildStore.addLog(buildId, "Uploading OCI bundle to Rivet...");

			const uploadResult = await uploadOCIBundleToRivet(
				conversionResult.bundleTarPath,
				build.buildName!,
				`${buildId}:latest`, // Match kaniko destination format
				rivetConfig,
				new Date().toISOString(), // Use timestamp as version for now
			);

			buildStore.addLog(
				buildId,
				`Successfully uploaded to Rivet: ${uploadResult.buildId}`,
			);
			buildStore.updateStatus(buildId, {
				type: "success",
				data: { buildId: uploadResult.buildId },
			});
		} finally {
			await conversionResult.cleanup();
		}
	} catch (error) {
		buildStore.updateStatus(buildId, {
			type: "failure",
			data: { reason: `Rivet upload process failed: ${error}` },
		});
		buildStore.addLog(buildId, `Rivet upload process failed: ${error}`);
		throw error;
	}
}

export async function createServer(port: number = 3000) {
	const app = new Hono();

	app.use(logger());

	const buildStore = new BuildStore();
	await buildStore.init();

	app.post("/builds", async (c) => {
		try {
			const formData = await c.req.formData();
			const buildName = formData.get("buildName") as string;
			const dockerfilePath = formData.get("dockerfilePath") as string;
			const environmentId = formData.get("environmentId") as string;
			const contextFile = formData.get("context") as File;

			if (!buildName) {
				return c.json({ error: "buildName is required" }, 400);
			}

			if (!dockerfilePath) {
				return c.json({ error: "dockerfilePath is required" }, 400);
			}

			if (!environmentId) {
				return c.json({ error: "environmentId is required" }, 400);
			}

			if (!contextFile) {
				return c.json({ error: "context file is required" }, 400);
			}

			// Create the build
			const buildId = buildStore.createBuild(buildName, dockerfilePath, environmentId);
			const contextPath = buildStore.getContextPath(buildId);

			if (!contextPath) {
				return c.json({ error: "Failed to create build" }, 500);
			}

			// Save the context file
			await mkdir(dirname(contextPath), { recursive: true });

			const contextBuffer = await contextFile.arrayBuffer();
			const fileStream = createWriteStream(contextPath);
			fileStream.write(new Uint8Array(contextBuffer));
			fileStream.end();

			buildStore.addLog(buildId, "Context uploaded successfully");

			// Run build in background
			buildStore.addLog(buildId, "About to start kaniko runner");

			const serverUrlParam = c.req.query("serverUrl");
			const serverUrl = serverUrlParam || new URL(c.req.url).origin;
			buildStore.addLog(buildId, `Using server URL: ${serverUrl}`);

			try {
				runKanikoBuild(buildStore, serverUrl, buildId).catch(
					(error) => {
						buildStore.addLog(
							buildId,
							`Failed to start build: ${error}`,
						);
						buildStore.updateStatus(buildId, {
							type: "failure",
							data: { reason: `Failed to start build: ${error}` },
						});
					},
				);
			} catch (error) {
				buildStore.addLog(
					buildId,
					`Sync error starting build: ${error}`,
				);
			}

			return c.json({ buildId });
		} catch (error) {
			return c.json({ error: "Failed to process build request" }, 500);
		}
	});

	app.get("/builds/:id", async (c) => {
		const buildId = c.req.param("id");
		const build = buildStore.getBuild(buildId);

		if (!build) {
			return c.json({ error: "Build not found" }, 404);
		}

		return c.json({
			id: build.id,
			status: build.status,
		});
	});

	app.get("/builds/:id/events", async (c) => {
		const buildId = c.req.param("id");
		const build = buildStore.getBuild(buildId);

		if (!build) {
			return c.json({ error: "Build not found" }, 404);
		}

		return streamSSE(c, async (stream) => {
			await stream.writeSSE({ data: "connected" });

			for (const event of build.events) {
				await stream.writeSSE({ data: JSON.stringify(event) });
			}

			const unsubscribe = buildStore.emitter.on(
				"build-event",
				async (eventBuildId, event) => {
					if (eventBuildId === buildId) {
						await stream.writeSSE({ data: JSON.stringify(event) });
					}
				},
			);

			let resolve: () => void;
			const abortPromise = new Promise<void>((res) => {
				resolve = res;
			});

			stream.onAbort(() => {
				unsubscribe();
				resolve();
			});

			await abortPromise;
		});
	});

	app.get("/builds/:id/kaniko/context.tar.gz", async (c) => {
		const buildId = c.req.param("id");
		console.log(`[SERVER] Kaniko requesting context for build ${buildId}`);
		const contextPath = buildStore.getContextPath(buildId);

		if (!contextPath) {
			return c.json({ error: "Build not found" }, 404);
		}

		try {
			const fileStream = createReadStream(contextPath);
			return new Response(fileStream as any, {
				headers: {
					"content-type": "application/gzip",
					"content-disposition":
						"attachment; filename=context.tar.gz",
				},
			});
		} catch (error) {
			return c.json({ error: "Context file not found" }, 404);
		}
	});

	app.get("/builds/:id/output.tar.gz", async (c) => {
		const buildId = c.req.param("id");
		const outputPath = buildStore.getOutputPath(buildId);

		if (!outputPath) {
			return c.json({ error: "Build not found" }, 404);
		}

		try {
			const fileStream = createReadStream(outputPath);

			// Mark as downloaded to trigger cleanup
			buildStore.markDownloaded(buildId);

			return new Response(fileStream as any, {
				headers: {
					"content-type": "application/gzip",
					"content-disposition": "attachment; filename=output.tar.gz",
				},
			});
		} catch (error) {
			return c.json({ error: "Output file not found" }, 404);
		}
	});

	app.put("/builds/:id/kaniko/output.tar.gz", async (c) => {
		const buildId = c.req.param("id");
		console.log(`[SERVER] Kaniko uploading output for build ${buildId}`);
		const outputPath = buildStore.getOutputPath(buildId);

		if (!outputPath) {
			return c.json({ error: "Build not found" }, 404);
		}

		try {
			await mkdir(dirname(outputPath), { recursive: true });

			const body = c.req.raw.body;
			if (!body) return c.json({ error: "Body does not exist" }, 400);

			const nodeStream = Readable.fromWeb(body as WebReadableStream<any>);
			const writeStream = createWriteStream(outputPath);
			await pipeline(nodeStream, writeStream);

			// Log upload details
			const stats = await stat(outputPath);
			buildStore.addLog(
				buildId,
				`Kaniko output uploaded successfully: ${outputPath} (${stats.size} bytes)`,
			);

			// Start Rivet upload process in background
			processRivetUpload(buildStore, buildId).catch((error) => {
				buildStore.addLog(buildId, `Rivet upload failed: ${error}`);
				buildStore.updateStatus(buildId, {
					type: "failure",
					data: { reason: `Rivet upload failed: ${error}` },
				});
			});

			return c.json({ message: "File saved successfully" });
		} catch (error) {
			return c.json({ error: "Failed to create output file" }, 500);
		}
	});

	return serve({
		fetch: app.fetch,
		port,
	});
}
