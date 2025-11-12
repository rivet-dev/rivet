import { z } from "zod";
import { getEnvUniversal } from "@/utils";

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

export const InspectorConfigSchema = z
	.object({
		enabled: z.boolean().default(defaultEnabled),

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
	}));
export type InspectorConfig = z.infer<typeof InspectorConfigSchema>;
