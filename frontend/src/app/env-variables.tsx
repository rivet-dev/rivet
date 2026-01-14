import { Button, CopyButton, DiscreteInput } from "@/components";
import { useEngineCompatDataProvider } from "@/components/actors";
import { Label } from "@/components/ui/label";
import { useAdminToken, usePublishableToken } from "@/queries/accessors";

export function EnvVariables({
	prefix,
	runnerName,
	endpoint,
	kind,
	prefixlessEndpoint = false,
}: {
	prefix?: string;
	runnerName?: string;
	endpoint: string;
	kind: "serverless" | "serverfull";
	prefixlessEndpoint?: boolean;
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

				{prefixlessEndpoint ? (
					<RivetEndpointEnv endpoint={endpoint} kind={kind} />
				) : null}
				<RivetEndpointEnv
					prefix={prefix}
					endpoint={endpoint}
					kind={kind}
				/>
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
	endpoint: string;
	kind: "serverless" | "serverfull";
}) => {
	const dataProvider = useEngineCompatDataProvider();
	const publishableToken = usePublishableToken();
	const adminToken = useAdminToken();
	const token = kind === "serverless" ? publishableToken : adminToken;

	const dsn = `https://${dataProvider.engineNamespace}:${token}@${endpoint
		.replace("https://", "")
		.replace("http://", "")}`;

	return dsn;
};

function RivetEndpointEnv({
	prefix,
	endpoint,
	kind,
}: {
	prefix?: string;
	endpoint: string;
	kind: "serverless" | "serverfull";
}) {
	const dsn = useRivetDsn({ endpoint, kind });
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
