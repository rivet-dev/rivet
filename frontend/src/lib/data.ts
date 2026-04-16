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

const rivetkitSchema = z
	.object({ version: z.string().optional() }).partial()

const metadataSchema = z
	.object({
		rivetkit: rivetkitSchema.or(z.string().optional().transform((str) => {
			try {
				if (typeof str !== "string") return undefined;
				const parsed = JSON.parse(str);
				return rivetkitSchema.parse(parsed);
			} catch {
				return undefined;
			}
		})).optional(),
	})
	.partial()
	.optional()

export function deriveRivetkitVersionFromMetadata(
	metadata: unknown,
): string | undefined {
	return metadataSchema
		.safeParse(metadata).data?.rivetkit?.version;
}

const safeJsonParse = (str: unknown): unknown => {
	if (typeof str !== "string") return str;
	try {
		return JSON.parse(str);
	} catch {
		return str;
	}
}