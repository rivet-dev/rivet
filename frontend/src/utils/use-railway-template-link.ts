import { useMemo } from "react";
import { useEndpoint } from "@/app/dialogs/connect-manual-serverfull-frame";
import { useRivetDsn } from "@/app/env-variables";

export function useRailwayTemplateLink({
	runnerName,
	kind,
}: {
	runnerName: string;
	kind: "serverless" | "serverfull";
}) {
	const endpoint = useEndpoint();
	const dsn = useRivetDsn({ endpoint, kind });

	return useMemo(() => {
		const url = new URL(
			"https://railway.com/new/template/rivet-cloud-starter",
		);
		url.searchParams.set("referralCode", "RC7bza");
		url.searchParams.set("utm_medium", "integration");
		url.searchParams.set("utm_source", "template");
		url.searchParams.set("utm_campaign", "generic");

		url.searchParams.set("RIVET_RUNNER", runnerName || "");
		if (dsn) {
			url.searchParams.set("RIVET_ENDPOINT", dsn);
		}

		return url.toString();
	}, [runnerName, dsn]);
}
