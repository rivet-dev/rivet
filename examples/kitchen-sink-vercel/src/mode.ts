export type KitchenSinkMode = "serverless" | "serverful" | "serverless-local";

export function resolveMode(): KitchenSinkMode {
	const explicit = process.env.RIVET_KITCHEN_SINK_MODE;
	if (
		explicit === "serverless" ||
		explicit === "serverful" ||
		explicit === "serverless-local"
	) {
		return explicit;
	}
	if (explicit !== undefined && explicit !== "") {
		throw new Error(
			`RIVET_KITCHEN_SINK_MODE must be one of "serverless", "serverful", or "serverless-local" (got "${explicit}")`,
		);
	}

	if (process.env.RIVET_RUN_ENGINE === "1") return "serverless-local";
	if (process.env.RIVET_SERVERLESS_URL !== undefined)
		return "serverless-local";
	if (process.env.KITCHEN_SINK_SERVERLESS_URL !== undefined) {
		return "serverless-local";
	}

	return "serverless";
}
