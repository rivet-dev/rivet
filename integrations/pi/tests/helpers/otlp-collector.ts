import { createServer } from "node:http";
import type { AddressInfo } from "node:net";

export interface CollectedSpan {
	name: string;
	traceId: string;
	spanId: string;
	attributes: Record<string, string | undefined>;
}

export interface OtlpCollector {
	readonly endpoint: string;
	spans(): CollectedSpan[];
	close(): Promise<void>;
}

interface OtlpPayload {
	resourceSpans?: Array<{
		scopeSpans?: Array<{
			spans?: Array<{
				name: string;
				traceId: string;
				spanId: string;
				attributes?: Array<{ key: string; value: { stringValue?: string } }>;
			}>;
		}>;
	}>;
}

/** Receives OTLP/JSON trace exports, such as RivetKit's native spans. */
export async function startOtlpCollector(): Promise<OtlpCollector> {
	const bodies: Buffer[] = [];
	const server = createServer((request, response) => {
		const chunks: Buffer[] = [];
		request.on("data", (chunk: Buffer) => chunks.push(chunk));
		request.on("end", () => {
			bodies.push(Buffer.concat(chunks));
			response.writeHead(200, { "content-type": "application/json" });
			response.end();
		});
	});
	await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
	const { port } = server.address() as AddressInfo;

	return {
		endpoint: `http://127.0.0.1:${port}/v1/traces`,
		spans: () =>
			bodies.flatMap((body) =>
				((JSON.parse(body.toString("utf8")) as OtlpPayload).resourceSpans ?? []).flatMap(
					(resource) =>
						(resource.scopeSpans ?? []).flatMap((scope) =>
							(scope.spans ?? []).map((span) => ({
								name: span.name,
								traceId: span.traceId,
								spanId: span.spanId,
								attributes: Object.fromEntries(
									(span.attributes ?? []).map(({ key, value }) => [
										key,
										value.stringValue,
									]),
								),
							})),
						),
				),
			),
		close: () =>
			new Promise<void>((resolve, reject) => {
				server.closeAllConnections();
				server.close((error) => (error ? reject(error) : resolve()));
			}),
	};
}
