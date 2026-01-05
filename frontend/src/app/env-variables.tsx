import { Button, CopyButton, DiscreteInput } from "@/components";
import { useEngineCompatDataProvider } from "@/components/actors";
import { Label } from "@/components/ui/label";
import { usePublishableToken } from "@/queries/accessors";

export function EnvVariables({
	prefix,
	runnerName,
	endpoint,
	unified = true,
}: {
	prefix?: string;
	runnerName?: string;
	endpoint: string;
	unified?: boolean;
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
				{unified ? (
					<RivetEndpointEnvUnified prefix={prefix} endpoint={endpoint} />
				) : (
					<>
						<RivetEndpointEnv prefix={prefix} endpoint={endpoint} />
						<RivetTokenEnv prefix={prefix} />
						<RivetNamespaceEnv prefix={prefix} />
					</>
				)}
				<RivetRunnerEnv prefix={prefix} runnerName={runnerName} />
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

function RivetEndpointEnvUnified({
	prefix,
	endpoint,
}: {
	prefix?: string;
	endpoint: string;
}) {
	const dataProvider = useEngineCompatDataProvider();
	const token = usePublishableToken();
	const namespace = dataProvider.engineNamespace || "default";
	
	// Create unified endpoint with embedded credentials
	const url = new URL(endpoint);
	url.username = encodeURIComponent(namespace);
	url.password = encodeURIComponent(token || "");
	const unifiedEndpoint = url.toString();
	
	return (
		<>
			<DiscreteInput
				aria-label="environment variable key"
				value={`${prefix ? `${prefix}_` : ""}RIVET_ENDPOINT`}
				show
			/>
			<DiscreteInput
				aria-label="environment variable value"
				value={unifiedEndpoint}
				show
			/>
		</>
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

function RivetTokenEnv({ prefix }: { prefix?: string }) {
	const data = usePublishableToken();
	return (
		<>
			<DiscreteInput
				aria-label="environment variable key"
				value={`${prefix ? `${prefix}_` : ""}RIVET_TOKEN`}
				show
			/>

			<DiscreteInput
				aria-label="environment variable value"
				value={(data as string) || ""}
				show
			/>
		</>
	);
}

function RivetEndpointEnv({
	prefix,
	endpoint,
}: {
	prefix?: string;
	endpoint: string;
}) {
	return (
		<>
			<DiscreteInput
				aria-label="environment variable key"
				value={`${prefix ? `${prefix}_` : ""}RIVET_ENDPOINT`}
				show
			/>
			<DiscreteInput
				aria-label="environment variable value"
				value={endpoint}
				show
			/>
		</>
	);
}

function RivetNamespaceEnv({ prefix }: { prefix?: string }) {
	const dataProvider = useEngineCompatDataProvider();
	return (
		<>
			<DiscreteInput
				aria-label="environment variable key"
				value={`${prefix ? `${prefix}_` : ""}RIVET_NAMESPACE`}
				show
			/>
			<DiscreteInput
				aria-label="environment variable value"
				value={dataProvider.engineNamespace || ""}
				show
			/>
		</>
	);
}
