import { execSync } from "child_process";
import { mkdir, rm, writeFile, readFile, readdir, stat } from "fs/promises";
import { join, dirname } from "path";
import { createReadStream, createWriteStream } from "fs";
import { pipeline } from "stream/promises";
import * as tar from "tar";

export interface OCIConversionResult {
	bundleTarPath: string;
	cleanup: () => Promise<void>;
}

export async function convertDockerTarToOCIBundle(
	dockerTarPath: string,
	tempDir: string = "/tmp/oci-conversion"
): Promise<OCIConversionResult> {
	const conversionId = Math.random().toString(36).substring(7);
	const workDir = join(tempDir, conversionId);
	
	try {
		await mkdir(workDir, { recursive: true });
		
		const dockerImagePath = join(workDir, "docker-image.tar");
		const ociImagePath = join(workDir, "oci-image");
		const ociBundlePath = join(workDir, "oci-bundle");
		const bundleTarPath = join(workDir, "oci-bundle.tar");

		// Extract docker tar if it's compressed
		const dockerTarData = await readFile(dockerTarPath);
		await writeFile(dockerImagePath, dockerTarData);

		// Convert Docker image to OCI image using skopeo
		console.log(`Converting Docker image to OCI image: ${dockerImagePath} -> ${ociImagePath}`);
		execSync(`skopeo copy docker-archive:${dockerImagePath} oci:${ociImagePath}:default`, {
			stdio: "pipe"
		});

		// Convert OCI image to OCI bundle using umoci
		console.log(`Converting OCI image to OCI bundle: ${ociImagePath} -> ${ociBundlePath}`);
		execSync(`umoci unpack --rootless --image ${ociImagePath}:default ${ociBundlePath}`, {
			stdio: "pipe"
		});

		// Create tar from OCI bundle
		console.log(`Creating tar from OCI bundle: ${ociBundlePath} -> ${bundleTarPath}`);
		await tar.create(
			{
				file: bundleTarPath,
				cwd: ociBundlePath,
			},
			["."]
		);

		// Clean up intermediate files
		await Promise.all([
			rm(dockerImagePath, { force: true }),
			rm(ociImagePath, { recursive: true, force: true }),
			rm(ociBundlePath, { recursive: true, force: true })
		]);

		const cleanup = async () => {
			try {
				await rm(workDir, { recursive: true, force: true });
			} catch (error) {
				console.warn(`Failed to cleanup OCI conversion directory ${workDir}:`, error);
			}
		};

		return {
			bundleTarPath,
			cleanup
		};
	} catch (error) {
		// Cleanup on error
		try {
			await rm(workDir, { recursive: true, force: true });
		} catch (cleanupError) {
			console.warn(`Failed to cleanup after error in ${workDir}:`, cleanupError);
		}
		throw new Error(`OCI conversion failed: ${error}`);
	}
}

