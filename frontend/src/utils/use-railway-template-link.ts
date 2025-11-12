import { useQuery } from "@tanstack/react-query";
import { useEngineCompatDataProvider } from "@/components/actors";
import { engineEnv } from "@/lib/env";

export function useRailwayTemplateLink({
	runnerName,
	datacenter,
}: {
	runnerName: string;
	datacenter: string;
}) {
	const dataProvider = useEngineCompatDataProvider();
	const { data: token } = useQuery(
		dataProvider.engineAdminTokenQueryOptions(),
	);
	const endpoint = useDatacenterEndpoint({ datacenter });

	return `https://railway.com/new/template/rivet-cloud-starter?referralCode=RC7bza&utm_medium=integration&utm_source=template&utm_campaign=generic&RIVET_TOKEN=${token || ""}&RIVET_ENDPOINT=${
		endpoint || ""
	}&RIVET_NAMESPACE=${
		dataProvider.engineNamespace || ""
	}&RIVET_RUNNER=${runnerName || ""}`;
}

const useDatacenterEndpoint = ({ datacenter }: { datacenter: string }) => {
	const { data } = useQuery(
		useEngineCompatDataProvider().datacenterQueryOptions(datacenter),
	);
	return data?.url || engineEnv().VITE_APP_API_URL;
};
