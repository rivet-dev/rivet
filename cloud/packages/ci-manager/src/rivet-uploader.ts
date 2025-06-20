import { RivetClient } from "@rivet-gg/api";
import { warn } from "console";
import { createReadStream, statSync } from "fs";
import { readFile } from "fs/promises";

export interface RivetUploadConfig {
	token: string;
	project?: string;
	environment?: string;
}

export interface RivetUploadResult {
	buildId: string;
}

export async function uploadOCIBundleToRivet(
	bundleTarPath: string,
	buildName: string,
	imageTag: string,
	config: RivetUploadConfig,
	buildVersion: string,
	extraTags?: Record<string, string>,
): Promise<RivetUploadResult> {
	const client = new RivetClient({ token: config.token });

	try {
		// Get file stats
		const stats = statSync(bundleTarPath);
		const fileSize = stats.size;

		console.log(
			`Preparing Rivet upload for ${bundleTarPath} (${fileSize} bytes)`,
		);

		// Prepare the build upload
		const prepareResponse = await client.builds.prepare({
			project: config.project,
			environment: config.environment,
			body: {
				imageTag,
				imageFile: {
					path: "oci-bundle.tar",
					contentType: "application/x-tar",
					contentLength: fileSize,
				},
				kind: "oci_bundle",
				compression: "none",
			},
		});

		const buildId = prepareResponse.build;
		const presignedRequests = prepareResponse.presignedRequests;

		console.log(`Rivet build prepared: ${buildId}`);
		console.log(`Upload chunks: ${presignedRequests.length}`);

		// Upload chunks in parallel
		const uploadPromises = presignedRequests.map(async (request, index) => {
			const { url, byteOffset, contentLength } = request;

			console.log(
				`Uploading chunk ${index + 1}/${presignedRequests.length}: offset=${byteOffset}, size=${contentLength}`,
			);

			// Read the specific chunk
			const buffer = Buffer.alloc(contentLength);
			const fileHandle = await import("fs/promises").then((fs) =>
				fs.open(bundleTarPath, "r"),
			);
			try {
				await fileHandle.read(buffer, 0, contentLength, byteOffset);
			} finally {
				await fileHandle.close();
			}

			// Upload chunk with retries
			await uploadChunkWithRetry(url, buffer, 3);

			console.log(
				`Chunk ${index + 1}/${presignedRequests.length} uploaded successfully`,
			);
		});

		// Wait for all chunks to upload
		await Promise.all(uploadPromises);

		console.log(`All chunks uploaded for build ${buildId}`);

		// Complete the build
		await client.builds.complete(buildId, {
			project: config.project,
			environment: config.environment,
		});

		console.log(`Rivet build completed: ${buildId}`);

		// Patch tags: remove "current" from existing builds with same name, then set current on new build
		await patchBuildTags(
			client,
			buildId,
			buildName,
			config,
			buildVersion,
			extraTags,
		);

		return { buildId };
	} catch (error) {
		throw new Error(`Rivet upload failed: ${error}`);
	}
}

async function uploadChunkWithRetry(
	url: string,
	buffer: Buffer,
	maxRetries: number = 3,
): Promise<void> {
	let lastError: Error | null = null;

	for (let attempt = 1; attempt <= maxRetries; attempt++) {
		try {
			const response = await fetch(url, {
				method: "PUT",
				body: buffer,
				headers: {
					"Content-Type": "application/octet-stream",
					"Content-Length": buffer.length.toString(),
				},
			});

			if (!response.ok) {
				throw new Error(
					`HTTP ${response.status}: ${response.statusText}`,
				);
			}

			return; // Success
		} catch (error) {
			lastError =
				error instanceof Error ? error : new Error(String(error));
			console.warn(
				`Upload attempt ${attempt}/${maxRetries} failed:`,
				lastError.message,
			);

			if (attempt < maxRetries) {
				// Exponential backoff: 1s, 2s, 4s
				const delay = Math.pow(2, attempt - 1) * 1000;
				await new Promise((resolve) => setTimeout(resolve, delay));
			}
		}
	}

	throw new Error(
		`Upload failed after ${maxRetries} attempts: ${lastError?.message}`,
	);
}

async function patchBuildTags(
	client: RivetClient,
	buildId: string,
	buildName: string,
	config: RivetUploadConfig,
	buildVersion: string,
	extraTags?: Record<string, string>,
): Promise<void> {
	try {
		console.log(
			`Patching tags for build ${buildId} with name: ${buildName}`,
		);

		// Step 1: Find existing builds with the same name and current=true
		const tagsFilter = JSON.stringify({
			name: buildName,
			current: "true",
		});

		const listResponse = await client.builds.list({
			project: config.project,
			environment: config.environment,
			tagsJson: tagsFilter,
		});

		console.log(
			`Found ${listResponse.builds.length} existing builds with current tag`,
		);

		// Step 2: Remove "current" tag from all existing builds with the same name
		for (const build of listResponse.builds) {
			try {
				await client.builds.patchTags(build.id, {
					project: config.project,
					environment: config.environment,
					body: {
						tags: {
							current: null,
						},
					},
				});
				console.log(`Removed current tag from build ${build.id}`);
			} catch (error) {
				console.warn(
					`Failed to remove current tag from build ${build.id}:`,
					error,
				);
			}
		}

		// Step 3: Set tags on the new build
		const tags: Record<string, string> = {
			name: buildName,
			version: buildVersion,
			current: "true",
		};

		// Add extra tags if provided
		if (extraTags) {
			Object.assign(tags, extraTags);
		}

		await client.builds.patchTags(buildId, {
			project: config.project,
			environment: config.environment,
			body: {
				tags,
			},
		});

		console.log(`Successfully patched tags for build ${buildId}`);
	} catch (error) {
		console.warn(`Failed to patch tags for build ${buildId}:`, error);
		// Don't throw here to avoid failing the entire upload process
	}
}

