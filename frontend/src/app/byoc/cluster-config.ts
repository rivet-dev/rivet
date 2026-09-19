export const CLUSTER_CONFIG_FILENAME = "rivet.auto.tfvars.json";

/** Non-secret Terraform inputs; the polling token is copied separately. */
export function serializeClusterConfig(
	clusterId: string | undefined,
	apiUrl: string,
): string | null {
	if (
		!clusterId ||
		!/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/.test(
			clusterId,
		)
	)
		return null;
	let origin: URL;
	try {
		origin = new URL(apiUrl);
	} catch {
		return null;
	}
	if (
		origin.protocol !== "https:" ||
		origin.username ||
		origin.password ||
		origin.search ||
		origin.hash ||
		origin.pathname !== "/"
	)
		return null;
	return (
		JSON.stringify(
			{
				byoc_cluster_id: clusterId,
				...(origin.origin === "https://cloud-api.rivet.dev"
					? {}
					: { cloud_api_url: origin.origin }),
			},
			null,
			2,
		) + "\n"
	);
}
