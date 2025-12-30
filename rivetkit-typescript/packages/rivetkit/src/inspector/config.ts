import { z } from "zod";
import {
	getRivetkitInspectorToken,
	isDev,
	getRivetkitInspectorDisable,
} from "@/utils/env-vars";

const defaultTokenFn = () => {
	const envToken = getRivetkitInspectorToken();

	if (envToken) {
		return envToken;
	}

	return "";
};

const defaultEnabled = () => {
	return (
		isDev() ||
		!getRivetkitInspectorDisable()
	);
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

		/**
		 * Token used to access the Inspector.
		 */
		token: z
			.custom<() => string>()
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
	}));
export type InspectorConfig = z.infer<typeof InspectorConfigSchema>;
