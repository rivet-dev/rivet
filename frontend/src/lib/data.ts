import z from "zod";

export function deriveProviderFromMetadata(metadata: unknown): string | undefined {
    return z.object({ provider: z.string().optional() }).partial().optional().parse(metadata)?.provider;
}
