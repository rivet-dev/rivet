import type { cors } from "hono/cors";
import { z } from "zod";
import { HEADER_ACTOR_QUERY } from "@/driver-helpers/mod";
import { getEnvUniversal } from "@/utils";

type CorsOptions = NonNullable<Parameters<typeof cors>[0]>;

const defaultTokenFn = () => {
	const envToken = getEnvUniversal("RIVETKIT_INSPECTOR_TOKEN");

	if (envToken) {
		return envToken;
	}

	return "";
};

const defaultEnabled = () => {
	return (
		getEnvUniversal("NODE_ENV") !== "production" ||
		!getEnvUniversal("RIVETKIT_INSPECTOR_DISABLE")
	);
};

const defaultInspectorOrigins = [
	"http://localhost:43708",
	"http://localhost:43709",
	"https://studio.rivet.gg",
	"https://inspect.rivet.dev",
	"https://dashboard.rivet.dev",
	"https://dashboard.staging.rivet.dev",
];

const defaultCors: CorsOptions = {
	origin: (origin) => {
		if (
			defaultInspectorOrigins.includes(origin) ||
			(origin.startsWith("https://") &&
				origin.endsWith("rivet-dev.vercel.app"))
		) {
			return origin;
		} else {
			return null;
		}
	},
	allowMethods: ["GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"],
	allowHeaders: [
		"Authorization",
		"Content-Type",
		"User-Agent",
		"baggage",
		"sentry-trace",
		"x-rivet-actor",
		"x-rivet-target",
	],
	maxAge: 3600,
	credentials: true,
};

export const InspectorConfigSchema = z
	.object({
		enabled: z
			.boolean()
			.or(
				z.object({
					actor: z.boolean().optional().default(true),
					manager: z.boolean().optional().default(true),
				}),
			)
			.optional()
			.default(defaultEnabled),
		/** CORS configuration for the router. Uses Hono's CORS middleware options. */
		cors: z
			.custom<CorsOptions>()
			.optional()
			.default(() => defaultCors),

		/**
		 * Token used to access the Inspector.
		 */
		token: z
			.function()
			.returns(z.string())
			.optional()
			.default(() => defaultTokenFn),

		/**
		 * Default RivetKit server endpoint for Rivet Inspector to connect to. This should be the same endpoint as what you use for your Rivet client to connect to RivetKit.
		 *
		 * This is a convenience property just for printing out the inspector URL.
		 */
		defaultEndpoint: z.string().optional(),
	})
	.optional()
	.default(() => ({
		enabled: defaultEnabled(),
		token: defaultTokenFn,
		cors: defaultCors,
	}));
export type InspectorConfig = z.infer<typeof InspectorConfigSchema>;
