import { z } from "zod/v4";

export const EnvoyConfigSchema = z
	.object({
		key: z.unknown().optional(),
		poolName: z.unknown().optional(),
		version: z.unknown().optional(),
		envoyKey: z.unknown().optional(),
	})
	.superRefine((config, ctx) => {
		if (config.key !== undefined) {
			ctx.addIssue({
				code: "custom",
				message: "envoy.key has been removed. Envoy keys are managed by RivetKit.",
				path: ["key"],
			});
		}
		if (config.poolName !== undefined) {
			ctx.addIssue({
				code: "custom",
				message: "envoy.poolName has been removed. Use top-level pool instead.",
				path: ["poolName"],
			});
		}
		if (config.version !== undefined) {
			ctx.addIssue({
				code: "custom",
				message: "envoy.version has been removed. Use top-level version instead.",
				path: ["version"],
			});
		}
		if (config.envoyKey !== undefined) {
			ctx.addIssue({
				code: "custom",
				message:
					"envoy.envoyKey has been removed. Envoy keys are managed by RivetKit.",
				path: ["envoyKey"],
			});
		}
	})
	.transform(() => ({}));

export type EnvoyConfigInput = z.input<typeof EnvoyConfigSchema>;
export type EnvoyConfig = z.infer<typeof EnvoyConfigSchema>;
