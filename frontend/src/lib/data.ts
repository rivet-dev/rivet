import z from "zod";

export function deriveProviderFromMetadata(
	metadata: unknown,
): string | undefined {
	return z
		.object({ provider: z.string().optional() })
		.partial()
		.optional()
		.parse(metadata)?.provider;
}

export function deriveRivetkitVersionFromMetadata(
	metadata: unknown,
): string | undefined {
	return z
		.object({
			rivetkit: z
				.object({ version: z.string().optional() })
				.partial()
				.optional(),
		})
		.partial()
		.optional()
		.parse(metadata)?.rivetkit?.version;
}
