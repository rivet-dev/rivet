/**
 * Build & push lz4. Most repos are on a significantly older version of lz4 that doesn't support multithreading.
 */

import { $, cd } from "zx";
import { mkdtempSync, mkdirSync, rmSync } from "fs";
import { tmpdir, platform } from "os";
import { join } from "path";
import { dirname } from "path";
import { fileURLToPath } from "url";

const LZ4_VERSION = "1.10.0";
const DISTRO = "debian11";
const UPLOAD_PATH = `tools/lz4/${LZ4_VERSION}/${DISTRO}-amd64/lz4`;

// Get the directory where this script is located
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

async function main() {
  // Get credentials
  const awsAccessKeyId = process.env.R2_RELEASES_ACCESS_KEY_ID || 
    (await $({ quiet: true })`op read "op://Engineering/rivet-releases R2 Upload/username"`).stdout.trim();
  const awsSecretAccessKey = process.env.R2_RELEASES_SECRET_ACCESS_KEY || 
    (await $({ quiet: true })`op read "op://Engineering/rivet-releases R2 Upload/password"`).stdout.trim();

  // Create temp directory for Docker build
  const tempDir = mkdtempSync(join(tmpdir(), 'lz4-docker-'));
  console.log(`Using temp directory: ${tempDir}`);
  
  // Create output directory
  const outputDir = join(tempDir, 'output');
  mkdirSync(outputDir, { recursive: true });

  // Build Docker image
  console.log("Building Docker image for Debian 11");
  await $`docker build -t lz4-builder:debian11 -f ${join(__dirname, 'Dockerfile.debian11')} --build-arg LZ4_VERSION=${LZ4_VERSION} ${__dirname}`;

  // Run container to extract binary
  console.log("Extracting LZ4 binary from container");
  await $`docker run --rm -v ${outputDir}:/output lz4-builder:debian11 cp /build/lz4-${LZ4_VERSION}/lz4 /output/`;

  console.log("LZ4 build complete");

  // Upload to S3
  console.log("Uploading LZ4 binary to S3");
  
  // Upload the binary
  console.log(`Uploading to s3://rivet-releases/${UPLOAD_PATH}`);
  await $({
	env: {
	  ...process.env,
	  AWS_ACCESS_KEY_ID: awsAccessKeyId,
	  AWS_SECRET_ACCESS_KEY: awsSecretAccessKey,
	  AWS_DEFAULT_REGION: "auto",
	}
  })`aws s3 cp ${join(outputDir, 'lz4')} ${`s3://rivet-releases/${UPLOAD_PATH}`} --content-type application/octet-stream --endpoint-url https://2a94c6a0ced8d35ea63cddc86c2681e7.r2.cloudflarestorage.com`;
  
  console.log("Upload complete!");
}

main().catch(error => {
  console.error(`Error: ${error}`);
  process.exit(1);
});
