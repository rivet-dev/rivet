import { match } from "ts-pattern";
import { Button, CopyButton, DiscreteInput, getConfig } from "@/components";
import { useEngineCompatDataProvider } from "@/components/actors";
import { Label } from "@/components/ui/label";
import { cloudEnv } from "@/lib/env";
import { useAdminToken, usePublishableToken } from "@/queries/accessors";

export function EnvVariables({
	runnerName,
	endpoint,
	showRunnerName = true,
	showEndpoint = true,
}: {
	runnerName?: string;
	endpoint?: string;
	showRunnerName?: boolean;
	showEndpoint?: boolean;
}) {
	return (
		<div>
			<div
				className="gap-1 items-center grid grid-cols-2"
				data-env-variables
			>
				<Label asChild className="text-muted-foreground text-xs mb-1">
					<p>Key</p>
				</Label>
				<Label asChild className="text-muted-foreground text-xs mb-1">
					<p>Value</p>
				</Label>

				{showEndpoint && <RivetPublicEndpointEnv endpoint={endpoint} />}
				{showEndpoint && <RivetRunnerEndpointEnv endpoint={endpoint} />}
				{showRunnerName && <RivetRunnerEnv runnerName={runnerName} />}
			</div>
			<div className="mt-2 flex justify-end">
				<CopyButton
					value={() => {
						const inputs =
							document.querySelectorAll<HTMLInputElement>(
								"[data-env-variables] input",
							);
						return Array.from(inputs)
							.reduce((acc, input, index) => {
								if (index % 2 === 0) {
									acc.push(
										`${input.value}=${inputs[index + 1]?.value}`,
									);
								}
								return acc;
							}, [] as string[])
							.join("\n");
					}}
				>
					<Button size="sm" variant="outline">
						Copy all raw
					</Button>
				</CopyButton>
			</div>
		</div>
	);
}

function RivetRunnerEnv({
	prefix,
	runnerName,
}: {
	prefix?: string;
	runnerName?: string;
}) {
	if (runnerName === "default") return null;

	return (
		<>
			<DiscreteInput
				aria-label="environment variable key"
				value={`${prefix ? `${prefix}_` : ""}RIVET_RUNNER`}
				show
			/>
			<DiscreteInput
				aria-label="environment variable value"
				value={runnerName || "default"}
				show
			/>
		</>
	);
}

export const useRivetDsn = ({
	endpoint,
	kind,
}: {
	endpoint?: string;
	kind: "publishable" | "secret";
}) => {
	const globalEndpoint = match(__APP_TYPE__)
		.with("cloud", () => cloudEnv().VITE_APP_API_URL)
		.with("engine", () => getConfig().apiUrl)
		.otherwise(() => getConfig().apiUrl);

	// Publishable (RIVET_PUBLIC_ENDPOINT) always uses global endpoint.
	// Secret (RIVET_ENDPOINT) uses regional endpoint if provided.
	const apiEndpoint =
		kind === "publishable" ? globalEndpoint : endpoint || globalEndpoint;

	const dataProvider = useEngineCompatDataProvider();
	const publishableToken = usePublishableToken();
	const adminToken = useAdminToken();
	const token = kind === "publishable" ? publishableToken : adminToken;

	const dsn = `https://${dataProvider.engineNamespace}:${token}@${apiEndpoint
		.replace("https://", "")
		.replace("http://", "")}`;

	return dsn;
};

export function RivetPublicEndpointEnv({
	prefix,
	endpoint,
}: {
	prefix?: string;
	endpoint?: string;
}) {
	const dsn = useRivetDsn({ endpoint, kind: "publishable" });
	return (
		<>
			<DiscreteInput
				aria-label="environment variable key"
				value={`${prefix ? `${prefix}_` : ""}RIVET_PUBLIC_ENDPOINT`}
				show
			/>
			<DiscreteInput
				aria-label="environment variable value"
				value={dsn}
				show
			/>
		</>
	);
}

export function RivetRunnerEndpointEnv({
	prefix,
	endpoint,
}: {
	prefix?: string;
	runnerName?: string;
	endpoint?: string;
}) {
	const dsn = useRivetDsn({ endpoint, kind: "secret" });
	return (
		<>
			<DiscreteInput
				aria-label="environment variable key"
				value={`${prefix ? `${prefix}_` : ""}RIVET_ENDPOINT`}
				show
			/>
			<DiscreteInput
				aria-label="environment variable value"
				value={dsn}
				show
			/>
		</>
	);
}
