import { faQuestionCircle, Icon } from "@rivet-gg/icons";
import { useInfiniteQuery, useQuery } from "@tanstack/react-query";
import { useRouteContext } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { match } from "ts-pattern";
import { HelpDropdown } from "@/app/help-dropdown";
import { PublishableTokenCodeGroup } from "@/app/publishable-token-code-group";
import {
	Button,
	CodePreview,
	type DialogContentProps,
	DiscreteInput,
	Frame,
	getConfig,
	Label,
	Skeleton,
} from "@/components";
import { RegionSelect } from "@/components/actors/region-select";
import { cloudEnv } from "@/lib/env";

/**
 * Creates a unified endpoint URL with embedded namespace and token credentials.
 */
function createUnifiedEndpoint(
	endpoint: string,
	namespace: string,
	token: string,
): string {
	const url = new URL(endpoint);
	url.username = encodeURIComponent(namespace);
	url.password = encodeURIComponent(token);
	return url.toString();
}

interface TokensFrameContentProps extends DialogContentProps {}

export default function TokensFrameContent({
	onClose,
}: TokensFrameContentProps) {
	return (
		<>
			<Frame.Header>
				<Frame.Title className="gap-2 flex items-center">
					<div>App Tokens</div>
					<HelpDropdown>
						<Button variant="ghost" size="icon">
							<Icon icon={faQuestionCircle} />
						</Button>
					</HelpDropdown>
				</Frame.Title>
				<Frame.Description>
					These tokens are used to authenticate your app with Rivet.
				</Frame.Description>
			</Frame.Header>
			<Frame.Content className="max-h-[70vh] overflow-y-auto">
				<div className="grid grid-cols-1 gap-8">
					<SecretToken />
					<PublishableToken />
				</div>
			</Frame.Content>
			<Frame.Footer>
				<Button variant="secondary" onClick={onClose}>
					Close
				</Button>
			</Frame.Footer>
		</>
	);
}

function SecretToken() {
	const dataProvider = useRouteContext({
		from: "/_context/_cloud/orgs/$organization/projects/$project/ns/$namespace",
		select: (c) => c.dataProvider,
	});
	const { data: token, isLoading: isTokenLoading } = useQuery(
		dataProvider.engineAdminTokenQueryOptions(),
	);
	const { data: regions = [] } = useInfiniteQuery(
		dataProvider.regionsQueryOptions(),
	);
	const [selectedDatacenter, setSelectedDatacenter] = useState<
		string | undefined
	>(undefined);

	// Set default datacenter when regions are loaded
	useEffect(() => {
		if (regions.length > 0 && !selectedDatacenter) {
			setSelectedDatacenter(regions[0].id);
		}
	}, [regions, selectedDatacenter]);

	const namespace = dataProvider.engineNamespace;

	const endpoint = match(__APP_TYPE__)
		.with("cloud", () => {
			const region = regions.find((r) => r.id === selectedDatacenter);
			return region?.url || cloudEnv().VITE_APP_API_URL;
		})
		.with("engine", () => getConfig().apiUrl)
		.otherwise(() => {
			throw new Error("Not in a valid context");
		});

	const envVars = `RIVET_ENDPOINT=${createUnifiedEndpoint(endpoint, namespace, token || "")}`;

	const codeSnippet = `// Configuration will automatically be read from env
registry.start();`;

	return (
		<div className="space-y-4">
			<div className="space-y-2">
				<Label>Secret Token</Label>
				{isTokenLoading ? (
					<Skeleton className="w-full h-10" />
				) : (
					<DiscreteInput value={token || ""} />
				)}
				<p className="text-sm text-muted-foreground">
					Only use in secure server environments. Grants full access
					to your namespace. Used to connect your Runners to your
					namespace.
				</p>
			</div>
			<div className="space-y-2">
				<Label>Datacenter</Label>
				<RegionSelect
					showAuto={false}
					value={selectedDatacenter}
					onValueChange={setSelectedDatacenter}
				/>
			</div>
			<div className="space-y-2">
				<Label>Environment Variables</Label>
				<CodePreview code={envVars} language="bash" />
			</div>
			<div className="space-y-2">
				<Label>Code Snippet</Label>
				<CodePreview code={codeSnippet} language="typescript" />
			</div>
		</div>
	);
}

function PublishableToken() {
	const dataProvider = useRouteContext({
		from: "/_context/_cloud/orgs/$organization/projects/$project/ns/$namespace",
		select: (c) => c.dataProvider,
	});
	const { data: token, isLoading } = useQuery(
		dataProvider.publishableTokenQueryOptions(),
	);

	const namespace = dataProvider.engineNamespace;

	const endpoint = match(__APP_TYPE__)
		.with("cloud", () => cloudEnv().VITE_APP_API_URL)
		.with("engine", () => getConfig().apiUrl)
		.otherwise(() => {
			throw new Error("Not in a valid context");
		});

	return (
		<div className="space-y-4">
			<div className="space-y-2">
				<Label>Publishable Token</Label>
				{isLoading ? (
					<Skeleton className="w-full h-10" />
				) : (
					<DiscreteInput value={token || ""} />
				)}
				<p className="text-sm text-muted-foreground">
					Safe to use in public contexts like client-side code. Allows
					your frontend to interact with Rivet services.
				</p>
			</div>
			{token && (
				<PublishableTokenCodeGroup
					token={token}
					endpoint={endpoint}
					namespace={namespace}
				/>
			)}
		</div>
	);
}
