import { Readable } from "node:stream";
import { GetObjectCommand, S3Client } from "@aws-sdk/client-s3";
import type { ActorContext } from "@rivet-gg/actor-core";
import { Hono } from "hono";

let s3Client: S3Client;

// Setup Hono app
const app = new Hono();

app.get("/health", (c) => {
	return c.text("ok");
});

app.get("/s3-test/:bucket/:key", async (c) => {
	try {
		const bucket = c.req.param("bucket");
		const key = c.req.param("key");

		console.log(`Starting S3 stream for ${bucket}/${key}`);

		// Get object from S3
		const response = await s3Client.send(
			new GetObjectCommand({
				Bucket: bucket,
				Key: key,
			}),
		);

		console.log(`Got S3 response for ${bucket}/${key}`);

		// Get content type from S3 response
		const contentType = response.ContentType || "application/octet-stream";

		// Convert S3 stream to Web stream with progress tracking
		const webStream1 = response.Body.transformToWebStream();
		console.log("Body type", webStream1.constructor.name);
		const s3Stream = Readable.from(webStream1);
		let bytesRead = 0;

		const webStream = new ReadableStream({
			start(controller) {
				console.log("Starting");
				s3Stream.on("data", (chunk) => {
					bytesRead += chunk.length;
					console.log(
						`Streaming progress for ${bucket}/${key}: ${bytesRead} bytes`,
					);
					controller.enqueue(chunk);
				});
				s3Stream.on("end", () => {
					console.log(
						`Completed streaming ${bucket}/${key}: ${bytesRead} total bytes`,
					);
					controller.close();
				});
				s3Stream.on("error", (err) => {
					console.error(`Error streaming ${bucket}/${key}:`, err);
					controller.error(err);
				});
			},
			cancel() {
				console.log(`Cancelled streaming ${bucket}/${key}`);
				s3Stream.destroy();
			},
		});

		// Return streaming response with proper content type and headers
		return new Response(webStream, {
			headers: {
				"Content-Type": contentType,
				"Transfer-Encoding": "chunked",
			},
		});
	} catch (error) {
		console.error("S3 streaming error:", error);
		return c.text("Error streaming from S3", 500);
	}
});

// Start server
export default {
	async start(ctx: ActorContext) {
		// Validate required environment variables
		const awsAccessKeyId = Deno.env.get("AWS_ACCESS_KEY_ID");
		const awsSecretAccessKey = Deno.env.get("AWS_SECRET_ACCESS_KEY");
		const awsRegion = Deno.env.get("AWS_REGION") || "us-east-1";
		const awsEndpoint = Deno.env.get("AWS_ENDPOINT");
		const portEnv = Deno.env.get("PORT_HTTP");

		if (!awsAccessKeyId) {
			throw new Error("missing AWS_ACCESS_KEY_ID");
		}
		if (!awsSecretAccessKey) {
			throw new Error("missing AWS_SECRET_ACCESS_KEY");
		}
		if (!portEnv) {
			throw new Error("missing PORT_HTTP");
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

		s3Client = new S3Client(s3Config);

		// Automatically exit after 1 minute in order to prevent accidental spam
		setTimeout(() => {
			console.error(
				"Actor should've been destroyed by now. Automatically exiting.",
			);
			Deno.exit(1);
		}, 60 * 1000);

		const port = Number.parseInt(portEnv);

		// Start server
		console.log(`Listening on port ${port}`);
		const server = Deno.serve({ port }, app.fetch);
		await server.finished;
	},
};
