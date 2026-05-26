import z from "zod";

const providerMetadataSchema = z
	.object({
		provider: z.string().optional(),
		customName: z.string().optional(),
		customIcon: z.string().optional(),
	})
	.partial()
	.optional();

export function deriveProviderFromMetadata(
	metadata: unknown,
): string | undefined {
	return providerMetadataSchema.safeParse(metadata).data?.provider;
}

// Minimal structural shapes for the cached infinite-query data we read in route
// loaders. We only touch the fields needed to decide onboarding state, so the
// real query types stay assignable to these.
export type RunnerConfigsInfiniteData = {
	pages: Array<{
		runnerConfigs: Record<
			string,
			{ datacenters: Record<string, { metadata?: unknown }> }
		>;
	}>;
};

export type RunnerNamesInfiniteData = {
	pages: Array<{ names: unknown[] }>;
};

// Decides which onboarding screen (if any) a namespace should show, given its
// runner state and actor count. Inputs may be undefined when only part of the
// runner state is available from cache.
export function deriveOnboardingState(opts: {
	runnerNames: RunnerNamesInfiniteData | undefined;
	runnerConfigs: RunnerConfigsInfiniteData | undefined;
	actorCount: number;
}) {
	const { runnerNames, runnerConfigs, actorCount } = opts;

	const provider = (runnerConfigs?.pages ?? [])
		.flatMap((page) =>
			Object.values(page.runnerConfigs).flatMap((config) =>
				Object.values(config.datacenters).map((dc) =>
					deriveProviderFromMetadata(dc.metadata),
				),
			),
		)
		.find((p) => p !== undefined);

	const hasRunnerNames = (runnerNames?.pages[0]?.names.length ?? 0) > 0;
	// A runner-config datacenter counts as "user-onboarded" only when
	// `metadata.provider` is set. Both onboarding signals write it: the
	// serverless `upsertRunnerConfig` call here in the frontend, and the
	// engine-ee compute-pool actor once GCP Cloud Run services come up. A
	// bare runner config without provider metadata (e.g. compute pre-enabled
	// out-of-band, or partial backend state) doesn't count, so the user
	// keeps seeing the provider/backend onboarding step instead of being
	// pushed straight to "Waiting for an Actor".
	const hasConfiguredDatacenter = (runnerConfigs?.pages ?? []).some((page) =>
		Object.values(page.runnerConfigs).some((config) =>
			Object.values(config.datacenters).some(
				(dc) => deriveProviderFromMetadata(dc.metadata) !== undefined,
			),
		),
	);
	const hasActors = actorCount > 0;
	const hasBackendConfigured = hasRunnerNames || hasConfiguredDatacenter;

	return {
		displayOnboarding: !hasBackendConfigured && !hasActors,
		displayFrontendOnboarding: hasBackendConfigured && !hasActors,
		provider,
	};
}

export function deriveCustomNameFromMetadata(
	metadata: unknown,
): string | undefined {
	const v = providerMetadataSchema.safeParse(metadata).data?.customName;
	return v?.trim() ? v.trim() : undefined;
}

export function deriveCustomIconFromMetadata(
	metadata: unknown,
): string | undefined {
	return providerMetadataSchema.safeParse(metadata).data?.customIcon ||
		undefined;
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