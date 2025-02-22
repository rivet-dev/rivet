import { GetObjectCommand, S3Client } from "@aws-sdk/client-s3";
//import { Readable } from "node:stream";
import { Readable } from "/Users/nathan/rivet/ee/oss/packages/toolchain/js-utils-embed/js/node_modules/unenv/runtime/node/stream/index.mjs";

// Helper to convert Node streams to Web streams
function nodeStreamToWebStream(nodeStream: Readable) {
	return new ReadableStream({
		start(controller) {
			console.log("start");
			nodeStream.on("data", (chunk) => {
				console.log("data");
				controller.enqueue(chunk);
			});
			nodeStream.on("end", () => {
				controller.close();
			});
			nodeStream.on("error", (err) => {
				console.log("error", err);
				controller.error(err);
			});
		},
		cancel() {
			nodeStream.destroy();
		},
	});
}

async function streamS3File(
	s3Client: S3Client,
	bucket: string,
	key: string,
): Promise<string> {
	try {
		// Get object from S3
		const response = await s3Client.send(
			new GetObjectCommand({
				Bucket: bucket,
				Key: key,
			}),
		);

		// Convert S3 stream to Web stream
		//console.log("Body type", response.Body?.constructor.name);
		//const s3Stream = Readable.from(response.Body.transformToWebStream());
		const webStream1 = response.Body.transformToWebStream();
		console.log("Body type", webStream1.constructor.name);
		const s3Stream = Readable.from(webStream1);
		const webStream = nodeStreamToWebStream(s3Stream);

		// Read the stream into a string
		const reader = webStream.getReader();
		const chunks: Uint8Array[] = [];

		while (true) {
			const { done, value } = await reader.read();
			if (done) break;
			chunks.push(value);
		}

		// Combine chunks and convert to string
		const allChunks = new Uint8Array(
			chunks.reduce((acc, chunk) => acc + chunk.length, 0),
		);
		let position = 0;
		for (const chunk of chunks) {
			allChunks.set(chunk, position);
			position += chunk.length;
		}

		return new TextDecoder().decode(allChunks);
	} catch (error) {
		console.error("S3 streaming error:", error);
		throw error;
	}
}

async function main() {
	const awsAccessKeyId = process.env.AWS_ACCESS_KEY_ID;
	const awsSecretAccessKey = process.env.AWS_SECRET_ACCESS_KEY;
	const awsRegion = process.env.AWS_REGION || "us-east-1";
	const awsEndpoint = process.env.AWS_ENDPOINT;

	if (!awsAccessKeyId) {
		throw new Error("missing AWS_ACCESS_KEY_ID");
	}
	if (!awsSecretAccessKey) {
		throw new Error("missing AWS_SECRET_ACCESS_KEY");
	}

	// Initialize S3 client
	const s3Config = {
		region: awsRegion,
		endpoint: awsEndpoint,
		credentials: {
			accessKeyId: awsAccessKeyId,
			secretAccessKey: awsSecretAccessKey,
		},
	};

	const s3Client = new S3Client(s3Config);

	const bucket = process.argv[2];
	const key = process.argv[3];

	if (!bucket || !key) {
		throw new Error("Usage: standalone_test.ts <bucket> <key>");
	}

	console.log(`Streaming from S3: ${bucket}/${key}`);

	try {
		const content = await streamS3File(s3Client, bucket, key);
		console.log("Content length:", content.length);
	} catch (error) {
		console.error("Error:", error);
		process.exit(1);
	}
}

main();

setInterval(() => {}, 1000);
