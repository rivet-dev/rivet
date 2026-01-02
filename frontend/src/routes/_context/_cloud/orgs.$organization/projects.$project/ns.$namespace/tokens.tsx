import {
	faChevronRight,
	faCopy,
	faNodeJs,
	faPlus,
	faQuestionCircle,
	faTrash,
	Icon,
} from "@rivet-gg/icons";
import { useInfiniteQuery, useMutation, useQuery } from "@tanstack/react-query";
import {
	createFileRoute,
	useParams,
	useRouteContext,
} from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { toast } from "sonner";
import { match } from "ts-pattern";
import { HelpDropdown } from "@/app/help-dropdown";
import { PublishableTokenCodeGroup } from "@/app/publishable-token-code-group";
import { SidebarToggle } from "@/app/sidebar-toggle";
import { useDialog } from "@/app/use-dialog";
import {
	Badge,
	Button,
	CodeFrame,
	CodeGroup,
	CodePreview,
	cn,
	DiscreteInput,
	DocsSheet,
	getConfig,
	H1,
	H3,
	Label,
	Skeleton,
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from "@/components";
import { useEngineCompatDataProvider } from "@/components/actors";
import { RegionSelect } from "@/components/actors/region-select";
import { useRootLayout } from "@/components/actors/root-layout-context";
import { cloudEnv } from "@/lib/env";
import { usePublishableToken } from "@/queries/accessors";
import { queryClient } from "@/queries/global";

export const Route = createFileRoute(
	"/_context/_cloud/orgs/$organization/projects/$project/ns/$namespace/tokens",
)({
	component: RouteComponent,
});

function RouteComponent() {
	const { isSidebarCollapsed } = useRootLayout();

	return (
		<div
			className={cn(
				" h-full overflow-auto @container",
				!isSidebarCollapsed && "bg-card border my-2 mr-2 rounded-lg",
			)}
		>
			<div className="max-w-5xl mx-auto">
				<div className="mt-2 flex justify-between items-center px-10 py-4">
					<SidebarToggle className="absolute left-4" />
					<H1>Tokens</H1>
					<HelpDropdown>
						<Button
							variant="outline"
							startIcon={<Icon icon={faQuestionCircle} />}
						>
							Need help?
						</Button>
					</HelpDropdown>
				</div>
				<p className="text-muted-foreground mb-6 px-10">
					These tokens are used to authenticate your app with Rivet.
				</p>
			</div>
			<hr className="mb-6" />
			<div className="px-4">
				<PublishableToken />
				<SecretToken />
				<CloudApiTokens />
			</div>
		</div>
	);
}

function PublishableToken() {
	const dataProvider = useEngineCompatDataProvider();
	const token = usePublishableToken();

	const namespace = dataProvider.engineNamespace;

	const endpoint = match(__APP_TYPE__)
		.with("cloud", () => cloudEnv().VITE_APP_API_URL)
		.with("engine", () => getConfig().apiUrl)
		.otherwise(() => {
			throw new Error("Not in a valid context");
		});

	return (
		<div className="pb-4 px-6 max-w-5xl mx-auto my-8 @6xl:border @6xl:rounded-lg bg-muted/10">
			<div className="flex gap-2 items-center mb-2 mt-6">
				<H3>Client Token</H3>
			</div>
			<p className="mb-6 text-muted-foreground">
				Connect to your actors using the Rivet client token. This can be
				used either on your frontend or backend. The code examples below show
				the unified endpoint format with embedded credentials (namespace:token@endpoint).
			</p>
			<div className="space-y-8">
				<DiscreteInput value={token || ""} show />

				{token ? (
					<PublishableTokenCodeGroup
						token={token}
						endpoint={endpoint}
						namespace={namespace}
					/>
				) : null}
			</div>
		</div>
	);
}

function SecretToken() {
	const dataProvider = useEngineCompatDataProvider();
	const { data: token, isLoading: isTokenLoading } = useQuery(
		dataProvider.engineAdminTokenQueryOptions(),
	);
	const { data: regions = [] } = useInfiniteQuery(
		dataProvider.regionsQueryOptions(),
	);
	const [selectedDatacenter, setSelectedDatacenter] = useState<
		string | undefined
	>(undefined);

	console.log(regions, selectedDatacenter);

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

	const codeSnippet = `import { registry } from "./registry";

// Automatically reads token from env
registry.start();`;

	return (
		<div className="pb-4 px-6 max-w-5xl mx-auto my-8 border-b @6xl:border @6xl:rounded-lg bg-muted/10">
			<div className="flex gap-2 items-center mb-2 mt-6">
				<H3>Runner Token</H3>
			</div>
			<p className="mb-6 text-muted-foreground">
				Used by runners (servers that run your actors) to authenticate
				with Rivet. Serverless providers do not need to use this token.
			</p>
			<div className="space-y-8">
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
					<div className="gap-1 items-center grid grid-cols-2">
						<Label
							asChild
							className="text-muted-foreground text-xs mb-1"
						>
							<p>Key</p>
						</Label>
						<Label
							asChild
							className="text-muted-foreground text-xs mb-1"
						>
							<p>Value</p>
						</Label>
						<DiscreteInput
							aria-label="environment variable key"
							value="RIVET_ENDPOINT"
							show
						/>
						<DiscreteInput
							aria-label="environment variable value"
							value={endpoint}
							show
						/>
						<DiscreteInput
							aria-label="environment variable key"
							value="RIVET_NAMESPACE"
							show
						/>
						<DiscreteInput
							aria-label="environment variable value"
							value={namespace}
							show
						/>
						<DiscreteInput
							aria-label="environment variable key"
							value="RIVET_TOKEN"
							show
						/>
						{isTokenLoading ? (
							<Skeleton className="w-full h-10" />
						) : (
							<DiscreteInput
								aria-label="environment variable value"
								value={token || ""}
							/>
						)}
					</div>
					<div className="flex justify-end">
						<Button
							variant="outline"
							size="sm"
							onClick={() => {
								const envVars = `RIVET_ENDPOINT=${endpoint}
RIVET_NAMESPACE=${namespace}
RIVET_TOKEN=${token || ""}`;
								navigator.clipboard.writeText(envVars);
								toast.success("Copied to clipboard");
							}}
						>
							Copy all raw
						</Button>
					</div>
				</div>
				<CodeGroup>
					{[
						<CodeFrame
							key="javascript"
							language="typescript"
							title="JavaScript"
							icon={faNodeJs}
							code={() => codeSnippet}
							footer={
								<DocsSheet
									path={"/docs/actors/quickstart/backend"}
									title={"JavaScript Quickstart"}
								>
									<span className="cursor-pointer hover:underline">
										See JavaScript Documentation{" "}
										<Icon
											icon={faChevronRight}
											className="text-xs"
										/>
									</span>
								</DocsSheet>
							}
						>
							<CodePreview
								className="w-full min-w-0"
								language="typescript"
								code={codeSnippet}
							/>
						</CodeFrame>,
					]}
				</CodeGroup>
			</div>
		</div>
	);
}

function CloudApiTokens() {
	const { dataProvider } = useRouteContext({
		from: "/_context/_cloud/orgs/$organization/projects/$project",
	});
	const params = useParams({ strict: false });
	const organization = params.organization;
	const project = params.project;
	const namespace = params.namespace;

	const { data, isLoading } = useQuery(dataProvider.apiTokensQueryOptions());

	const { open: openCreateApiToken, dialog: createApiTokenDialog } =
		useDialog.CreateApiToken({});

	const cloudApiUrl = cloudEnv().VITE_APP_CLOUD_API_URL;

	return (
		<div className="pb-4 px-6 max-w-5xl mx-auto my-8 @6xl:border @6xl:rounded-lg bg-muted/10">
			<div className="flex gap-2 items-center justify-between mb-2 mt-6">
				<div className="flex gap-2 items-center">
					<H3>Cloud API Tokens</H3>
					<Badge variant="secondary">Beta</Badge>
				</div>
				<Button
					className="min-w-32"
					variant="outline"
					onClick={() => openCreateApiToken()}
					startIcon={<Icon icon={faPlus} />}
				>
					Create API Token
				</Button>
			</div>
			<p className="mb-6 text-muted-foreground">
				Cloud API tokens provide programmatic access to the Rivet Cloud
				API. Keep them secure and never share them publicly.
			</p>
			<div className="border rounded-md">
				{isLoading ? (
					<div className="space-y-2 p-4">
						<Skeleton className="w-full h-12" />
						<Skeleton className="w-full h-12" />
						<Skeleton className="w-full h-12" />
					</div>
				) : (
					<Table>
						<TableHeader>
							<TableRow>
								<TableHead>Name</TableHead>
								<TableHead>Token</TableHead>
								<TableHead>Created</TableHead>
								<TableHead>Expires</TableHead>
								<TableHead w="min" />
							</TableRow>
						</TableHeader>
						<TableBody>
							{data?.apiTokens.length === 0 ? (
								<TableRow>
									<TableCell
										colSpan={5}
										className="text-center py-8 text-muted-foreground"
									>
										No Cloud API tokens yet. Create one to
										get started.
									</TableCell>
								</TableRow>
							) : (
								data?.apiTokens.map((apiToken) => (
									<ApiTokenRow
										key={apiToken.id}
										apiToken={apiToken}
										dataProvider={dataProvider}
									/>
								))
							)}
						</TableBody>
					</Table>
				)}
			</div>
			<div className="space-y-4 mt-8">
				<CodeGroup>
					<CodeFrame
						language="typescript"
						title="Create Namespace"
						code={() => `const response = await fetch("${cloudApiUrl}/projects/${project}/namespaces?org=${organization}", {
  method: "POST",
  headers: {
    "Authorization": "Bearer \${YOUR_CLOUD_API_TOKEN}",
    "Content-Type": "application/json"
  },
  body: JSON.stringify({
    displayName: "my-namespace"
  })
});

const data = await response.json();
console.log(data.namespace);`}
					>
						<CodePreview
							className="w-full min-w-0"
							language="typescript"
							code={`const response = await fetch("${cloudApiUrl}/projects/${project}/namespaces?org=${organization}", {
  method: "POST",
  headers: {
    "Authorization": "Bearer \${YOUR_CLOUD_API_TOKEN}",
    "Content-Type": "application/json"
  },
  body: JSON.stringify({
    displayName: "my-namespace"
  })
});

const data = await response.json();
console.log(data.namespace);`}
						/>
					</CodeFrame>
					<CodeFrame
						language="typescript"
						title="List Namespaces"
						code={() => `const response = await fetch("${cloudApiUrl}/projects/${project}/namespaces?org=${organization}&limit=10", {
  method: "GET",
  headers: {
    "Authorization": "Bearer \${YOUR_CLOUD_API_TOKEN}"
  }
});

const data = await response.json();
console.log(data.namespaces);`}
					>
						<CodePreview
							className="w-full min-w-0"
							language="typescript"
							code={`const response = await fetch("${cloudApiUrl}/projects/${project}/namespaces?org=${organization}&limit=10", {
  method: "GET",
  headers: {
    "Authorization": "Bearer \${YOUR_CLOUD_API_TOKEN}"
  }
});

const data = await response.json();
console.log(data.namespaces);`}
						/>
					</CodeFrame>
					<CodeFrame
						language="typescript"
						title="Get Namespace"
						code={() => `const response = await fetch("${cloudApiUrl}/projects/${project}/namespaces/${namespace}?org=${organization}", {
  method: "GET",
  headers: {
    "Authorization": "Bearer \${YOUR_CLOUD_API_TOKEN}"
  }
});

const data = await response.json();
console.log(data.namespace);`}
					>
						<CodePreview
							className="w-full min-w-0"
							language="typescript"
							code={`const response = await fetch("${cloudApiUrl}/projects/${project}/namespaces/${namespace}?org=${organization}", {
  method: "GET",
  headers: {
    "Authorization": "Bearer \${YOUR_CLOUD_API_TOKEN}"
  }
});

const data = await response.json();
console.log(data.namespace);`}
						/>
					</CodeFrame>
					<CodeFrame
						language="typescript"
						title="Create Runner Token"
						code={() => `const response = await fetch("${cloudApiUrl}/projects/${project}/namespaces/${namespace}/tokens/secret?org=${organization}", {
  method: "POST",
  headers: {
    "Authorization": "Bearer \${YOUR_CLOUD_API_TOKEN}"
  }
});

const data = await response.json();
console.log(data.token);`}
					>
						<CodePreview
							className="w-full min-w-0"
							language="typescript"
							code={`const response = await fetch("${cloudApiUrl}/projects/${project}/namespaces/${namespace}/tokens/secret?org=${organization}", {
  method: "POST",
  headers: {
    "Authorization": "Bearer \${YOUR_CLOUD_API_TOKEN}"
  }
});

const data = await response.json();
console.log(data.token);`}
						/>
					</CodeFrame>
					<CodeFrame
						language="typescript"
						title="Create Client Token"
						code={() => `const response = await fetch("${cloudApiUrl}/projects/${project}/namespaces/${namespace}/tokens/publishable?org=${organization}", {
  method: "POST",
  headers: {
    "Authorization": "Bearer \${YOUR_CLOUD_API_TOKEN}"
  }
});

const data = await response.json();
console.log(data.token);`}
					>
						<CodePreview
							className="w-full min-w-0"
							language="typescript"
							code={`const response = await fetch("${cloudApiUrl}/projects/${project}/namespaces/${namespace}/tokens/publishable?org=${organization}", {
  method: "POST",
  headers: {
    "Authorization": "Bearer \${YOUR_CLOUD_API_TOKEN}"
  }
});

const data = await response.json();
console.log(data.token);`}
						/>
					</CodeFrame>
				</CodeGroup>
			</div>
			{createApiTokenDialog}
		</div>
	);
}

interface ApiTokenRowProps {
	apiToken: {
		id: string;
		name: string;
		createdAt: string;
		expiresAt?: string;
		revoked: boolean;
		lastFourChars: string;
	};
	dataProvider: ReturnType<
		typeof useRouteContext<"/_context/_cloud/orgs/$organization/projects/$project">
	>["dataProvider"];
}

function ApiTokenRow({ apiToken, dataProvider }: ApiTokenRowProps) {
	const { mutate: revoke, isPending } = useMutation(
		dataProvider.revokeApiTokenMutationOptions({
			onSuccess: async () => {
				await queryClient.invalidateQueries(
					dataProvider.apiTokensQueryOptions(),
				);
			},
		}),
	);

	const createdDate = new Date(apiToken.createdAt).toLocaleDateString();
	const expiresDate = apiToken.expiresAt
		? new Date(apiToken.expiresAt).toLocaleDateString()
		: "Never";

	return (
		<TableRow className={apiToken.revoked ? "opacity-50" : ""}>
			<TableCell>{apiToken.name}</TableCell>
			<TableCell>
				<code className="text-xs bg-muted px-2 py-1 rounded">
					cloud_api_...{apiToken.lastFourChars}
				</code>
			</TableCell>
			<TableCell>{createdDate}</TableCell>
			<TableCell>{expiresDate}</TableCell>
			<TableCell>
				{!apiToken.revoked && (
					<Button
						variant="destructive"
						size="sm"
						isLoading={isPending}
						onClick={() => {
							revoke({ apiTokenId: apiToken.id });
						}}
					>
						<Icon icon={faTrash} className="mr-1" />
						Revoke
					</Button>
				)}
				{apiToken.revoked && (
					<span className="text-muted-foreground text-sm">
						Revoked
					</span>
				)}
			</TableCell>
		</TableRow>
	);
}
