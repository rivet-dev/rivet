import * as Sentry from "@sentry/react";
import { z } from "zod";

function getModuleCallOrNull(event: Event, url: string) {
	if (event.request.method !== "POST") {
		return null;
	}

	const [
		modulesLiteral,
		moduleName,
		scriptsLiteral,
		scriptName,
		callLiteral,
	] = url.split("/").slice(1);

	if (
		modulesLiteral === "modules" &&
		scriptsLiteral === "scripts" &&
		callLiteral === "call"
	) {
		return {
			moduleName,
			scriptName,
		};
	}

	return null;
}

const Event = z.object({
	request: z.object({
		headers: z.record(z.string()),
		method: z.string(),
		url: z.string(),
	}),
	response: z.object({
		status: z.number(),
	}),
});

type Event = z.infer<typeof Event>;

export const BackendEvent = z
	.object({
		dispatchEnvironment: z.string().optional(),
		event: Event,
		eventTimestamp: z.string(),
		logs: z
			.array(
				z.object({
					level: z.string(),
					message: z.array(z.string()),
					timestamp: z.string(),
				}),
			)
			.optional(),
		exceptions: z
			.array(
				z.object({
					stack: z.string().optional(),
					message: z.string(),
					timestamp: z.string(),
				}),
			)
			.optional(),
		outcome: z
			.literal("canceled")
			.or(z.literal("exceededCpu"))
			.or(z.string()),
		scriptName: z.string(),
		scriptVersion: z.object({
			id: z.string(),
		}),
	})
	.catch((ctx) => {
		console.error(ctx.error);
		Sentry.captureException(ctx.error);
		return ctx.input;
	})
	.transform((data) => {
		const url = new URL(data.event.request.url);

		const backendCall = getModuleCallOrNull(data.event, url.pathname);

		return {
			...data,
			backendCall,
			eventDate: new Date(+data.eventTimestamp).toLocaleString(),
			event: {
				...data.event,
				request: {
					...data.event.request,
					pathname: url.pathname,
					fmtUrl: backendCall
						? `${backendCall.moduleName}.${backendCall.scriptName}`
						: url.pathname,
				},
			},
			logTimestamps: data.logs
				? [
						...data.logs.map((log) =>
							new Date(+log.timestamp).toISOString(),
						),
						...(data.exceptions?.map((log) =>
							new Date(+log.timestamp).toISOString(),
						) ?? []),
					]
				: [],
			logs: data.logs
				? [
						...data.logs.map((log) => ({
							type: log.level as "error" | "warn" | "log",
							message: log.message.join("\n"),
						})),
						...(data.exceptions?.map((log) => ({
							type: "error" as const,
							message: [log.message, log.stack].join("\n"),
						})) ?? []),
					]
				: [],
		};
	});

export type BackendEvent = z.infer<typeof BackendEvent>;

export const OuterbaseStarlinkResponse = z.object({
	response: z.object({
		url: z.string(),
	}),
});
export type OuterbaseStarlinkResponse = z.infer<
	typeof OuterbaseStarlinkResponse
>;
