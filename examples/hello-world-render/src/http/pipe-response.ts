import type { ServerResponse } from "node:http";
import { pipeline } from "node:stream/promises";
import { Readable } from "node:stream";

function isStreamPrematureClose(err: unknown): boolean {
	return (
		typeof err === "object" &&
		err !== null &&
		"code" in err &&
		(err as NodeJS.ErrnoException).code === "ERR_STREAM_PREMATURE_CLOSE"
	);
}

/** Bridge Fetch `Response` (incl. SSE from `GET /api/rivet/start`) to Node `ServerResponse`. */
export async function pipeWebResponseToNode(
	nodeRes: ServerResponse,
	webRes: Response,
): Promise<void> {
	nodeRes.statusCode = webRes.status;
	webRes.headers.forEach((value, key) => {
		if (key.toLowerCase() === "transfer-encoding") return;
		nodeRes.setHeader(key, value);
	});
	if (webRes.body) {
		try {
			await pipeline(
				Readable.fromWeb(webRes.body as import("stream/web").ReadableStream),
				nodeRes,
			);
		} catch (err) {
			if (isStreamPrematureClose(err)) return;
			throw err;
		}
	} else {
		nodeRes.end();
	}
}
