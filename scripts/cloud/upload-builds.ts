#!/usr/bin/env tsx

/**
 * Build and upload CI manager and runner Docker images
 */

import { $, cd } from "zx";
import { mkdtempSync, mkdirSync, rmSync } from "fs";
import { tmpdir } from "os";
import { join } from "path";
import { dirname } from "path";
import { fileURLToPath } from "url";

// Get the directory where this script is located
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const projectRoot = join(__dirname, '../../');

// Generate ISO date with milliseconds for upload path
const isoDate = new Date().toISOString().replace(/[:\.T]/g, "-");

const COMPONENTS = ['ci-manager', 'ci-runner'];

async function main() {
  // Get credentials
  const awsAccessKeyId = process.env.R2_RELEASES_ACCESS_KEY_ID || 
    (await $({ quiet: true })`op read "op://Engineering/rivet-releases R2 Upload/username"`).stdout.trim();
  const awsSecretAccessKey = process.env.R2_RELEASES_SECRET_ACCESS_KEY || 
    (await $({ quiet: true })`op read "op://Engineering/rivet-releases R2 Upload/password"`).stdout.trim();

  // Create temp directory
  const tempDir = mkdtempSync(join(tmpdir(), 'ci-builds-'));
  console.log(`Using temp directory: ${tempDir}`);
  
  const uploadedUrls: string[] = [];
  
  try {
    for (const component of COMPONENTS) {
      console.log(`\nBuilding ${component}...`);
      
      const componentPath = join(projectRoot, 'cloud/packages', component);
      const tarPath = join(tempDir, `${component}.tar`);
      
      // Build Docker image
      console.log(`Building Docker image for ${component}`);
      await $`docker build -t ${component}:latest ${componentPath}`;
      
      // Save Docker image to tar file
      console.log(`Saving ${component} image to tar file`);
      await $`docker save -o ${tarPath} ${component}:latest`;
      
      // Upload to S3
      const uploadPath = `${component}/${isoDate}/image.tar`;
      console.log(`Uploading ${component} to s3://rivet-releases/${uploadPath}`);
      
      await $({
        env: {
          ...process.env,
          AWS_ACCESS_KEY_ID: awsAccessKeyId,
          AWS_SECRET_ACCESS_KEY: awsSecretAccessKey,
          AWS_DEFAULT_REGION: "auto",
        }
      })`aws s3 cp ${tarPath} s3://rivet-releases/${uploadPath} --content-type application/octet-stream --endpoint-url https://2a94c6a0ced8d35ea63cddc86c2681e7.r2.cloudflarestorage.com`;
      
      const url = `https://releases.rivet.gg/${uploadPath}`;
      uploadedUrls.push(url);
      console.log(`${component} upload complete!`);
    }
  } finally {
    // Clean up temp directory
    console.log(`Cleaning up temp directory: ${tempDir}`);
    rmSync(tempDir, { recursive: true, force: true });
  }
  
  console.log("\nAll builds and uploads complete!");
  console.log("\nUploaded images:");
  uploadedUrls.forEach(url => console.log(`  ${url}`));
}

main().catch(error => {
  console.error(`Error: ${error}`);
  process.exit(1);
});
