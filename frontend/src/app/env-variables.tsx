import { useSuspenseQuery } from "@tanstack/react-query";
import { useRouteContext } from "@tanstack/react-router";
import { match } from "ts-pattern";
import { Button, CopyButton, DiscreteInput } from "@/components";
import {
	useEngineCompatDataProvider,
	useEngineNamespaceDataProvider,
} from "@/components/actors";
import { Label } from "@/components/ui/label";

export function EnvVariables({
	prefix,
	runnerName,
	endpoint,
}: {
	prefix?: string;
	runnerName?: string;
	endpoint: string;
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
				<RivetEndpointEnv prefix={prefix} endpoint={endpoint} />
				<RivetTokenEnv prefix={prefix} />
				<RivetNamespaceEnv prefix={prefix} />
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

export function usePublishableToken() {
	return match(__APP_TYPE__)
		.with("cloud", () => {
			// biome-ignore lint/correctness/useHookAtTopLevel: guarded by the build flag
			const routeContext = useRouteContext({
				from: "/_context/_cloud/orgs/$organization/projects/$project/ns/$namespace/connect",
				select: (ctx) => ctx.dataProvider,
			});
			// biome-ignore lint/correctness/useHookAtTopLevel: guarded by the build flag
			return useSuspenseQuery(routeContext.publishableTokenQueryOptions())
				.data;
		})
		.with("engine", () => {
			// biome-ignore lint/correctness/useHookAtTopLevel: guarded by the build flag
			return useSuspenseQuery(
				// biome-ignore lint/correctness/useHookAtTopLevel: guarded by the build flag
				useEngineNamespaceDataProvider().engineAdminTokenQueryOptions(),
			).data;
		})
		.otherwise(() => {
			throw new Error("Not in a valid context");
		});
}
