import { AccordionItem } from "@radix-ui/react-accordion";
import {
	faChevronRight,
	faHono,
	faNodeJs,
	faPlus,
	faQuestionCircle,
	faTrash,
	Icon,
} from "@rivet-gg/icons";
import {
	useMutation,
	usePrefetchInfiniteQuery,
	useQuery,
	useSuspenseInfiniteQuery,
} from "@tanstack/react-query";
import {
	createFileRoute,
	redirect,
	useParams,
	useRouteContext,
} from "@tanstack/react-router";
import { useState } from "react";
import { match } from "ts-pattern";
import { useEndpoint } from "@/app/dialogs/connect-manual-serverfull-frame";
import { EnvVariables, useRivetDsn } from "@/app/env-variables";
import { HelpDropdown } from "@/app/help-dropdown";
import { PublishableTokenCodeGroup } from "@/app/publishable-token-code-group";
import { SidebarToggle } from "@/app/sidebar-toggle";
import { useDialog } from "@/app/use-dialog";
import {
	Accordion,
	AccordionContent,
	AccordionTrigger,
	Badge,
	Button,
	CodeFrame,
	CodeGroup,
	CodePreview,
	cn,
	DiscreteInput,
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
	Tabs,
	TabsContent,
	TabsList,
	TabsTrigger,
} from "@/components";
import {
	useDataProvider,
	useEngineCompatDataProvider,
} from "@/components/actors";
import { RegionSelect } from "@/components/actors/region-select";
import { useRootLayout } from "@/components/actors/root-layout-context";
import { docsLinks } from "@/content/data";
import { cloudEnv } from "@/lib/env";
import { queryClient } from "@/queries/global";

export const Route = createFileRoute(
	"/_context/_cloud/orgs/$organization/projects/$project/ns/$namespace/tokens",
)({
	beforeLoad: async ({ context, params }) => {
		throw redirect({
			to: "/orgs/$organization/projects/$project/ns/$namespace/settings",
			params,
			search: true,
		});
	},
	component: RouteComponent,
});

function RouteComponent() {
	const { isSidebarCollapsed } = useRootLayout();

	const dataProvider = useDataProvider();

	usePrefetchInfiniteQuery(dataProvider.datacentersQueryOptions());

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
			<div className="px-4 max-w-5xl mx-auto">
				<SecretToken />
				<Accordion type="single" collapsible>
					<AccordionItem value="advanced">
						<AccordionTrigger>Advanced</AccordionTrigger>
						<AccordionContent>
							<PublishableToken />
							<CloudApiTokens />
						</AccordionContent>
					</AccordionItem>
				</Accordion>
			</div>
		</div>
	);
}

export function PublishableToken() {
	const dsn = useRivetDsn({ endpoint: "", kind: "publishable" });

	return (
		<div className="pb-4 pb-8 px-6 max-w-5xl mx-auto my-8 border-b @6xl:border @6xl:rounded-lg ">
			<div className="flex gap-2 items-center mb-2 mt-6">
				<H3>Manual Client Configuration</H3>
			</div>
			<p className="mb-6 text-muted-foreground">
				Manually configuring the client is only required for{" "}
				<a
					href="https://www.rivet.dev/docs/general/runtime-modes/#runners"
					target="_blank"
					rel="noopener noreferrer"
					className="underline"
				>
					Runner Runtime Mode
				</a>{" "}
				or clients that need to be configured to connect directly to
				Rivet.
			</p>
			<div className="space-y-8">
				<DiscreteInput value={dsn || ""} show />

				<PublishableTokenCodeGroup />
			</div>
		</div>
	);
}

export function SecretToken() {
	return (
		<div className="pb-4 pb-8 px-6 max-w-5xl mx-auto my-8 border-b @6xl:border @6xl:rounded-lg">
			<div className="flex gap-2 items-center mb-2 mt-6">
				<H3>Backend Configuration</H3>
			</div>
			<p className="mb-6 text-muted-foreground">
				Used by Rivet to run your actors. Choose between Serverless
				mode, where Rivet sends HTTP requests to your backend, or
				Runners mode, where Rivet runs your actors as long-running
				background processes.
			</p>

			<Tabs defaultValue="serverless">
				<TabsList>
					<TabsTrigger value="serverless">Serverless</TabsTrigger>
					<TabsTrigger value="runners">Runners</TabsTrigger>
				</TabsList>
				<TabsContent value="serverless">
					<ServerlessModeInfo />
				</TabsContent>
				<TabsContent value="runners">
					<RunnersModeInfo />
				</TabsContent>
			</Tabs>
		</div>
	);
}

const serverlessCodeSnippet = `import { registry } from "./registry";

export default registry.serve();`;

const serverlessCodeSnippetHono = `import { Hono } from "hono";
import { registry } from "./registry";

const app = new Hono();

app.all("/api/rivet/*", (c) => registry.handler(c.req.raw));`;

export function ServerlessModeInfo() {
	return (
		<div className="space-y-8">
			<p>
				Serverless is the default and recommended mode. Rivet sends HTTP
				requests to your backend to run actor logic, allowing your
				infrastructure to scale automatically. Read more about{" "}
				<b>Runtime Modes</b> in the
				<a
					href={docsLinks.runtimeModes}
					target="_blank"
					rel="noopener noreferrer"
					className="ml-1 underline"
				>
					documentation
				</a>
				.
			</p>
			<div className="space-y-2">
				<Label>Environment Variables</Label>
				<EnvVariables endpoint={""} showRunnerName={false} />
			</div>
			<CodeGroup>
				{[
					<CodeFrame
						key="javascript"
						language="typescript"
						title="Direct"
						icon={faNodeJs}
						code={() => serverlessCodeSnippet}
						footer={
							<a
								href={docsLinks.quickstart.backend}
								target="_blank"
								rel="noopener noreferrer"
							>
								<span className="cursor-pointer hover:underline">
									See JavaScript Documentation{" "}
									<Icon
										icon={faChevronRight}
										className="text-xs"
									/>
								</span>
							</a>
						}
					>
						<CodePreview
							className="w-full min-w-0"
							language="typescript"
							code={serverlessCodeSnippet}
						/>
					</CodeFrame>,
					<CodeFrame
						key="hono"
						language="typescript"
						title="Hono"
						icon={faHono}
						code={() => serverlessCodeSnippetHono}
						footer={
							<a
								href={docsLinks.quickstart.backend}
								target="_blank"
								rel="noopener noreferrer"
							>
								<span className="cursor-pointer hover:underline">
									See JavaScript Documentation{" "}
									<Icon
										icon={faChevronRight}
										className="text-xs"
									/>
								</span>
							</a>
						}
					>
						<CodePreview
							className="w-full min-w-0"
							language="typescript"
							code={serverlessCodeSnippetHono}
						/>
					</CodeFrame>,
				]}
			</CodeGroup>
		</div>
	);
}

function RunnersModeInfo() {
	const dataProvider = useEngineCompatDataProvider();
	const { data: regions = [] } = useSuspenseInfiniteQuery(
		dataProvider.datacentersQueryOptions(),
	);
	const [selectedDatacenter, setSelectedDatacenter] = useState<string>(
		() => regions[0]?.name,
	);

	const endpoint = match(__APP_TYPE__)
		.with("cloud", () => {
			const region = regions.find((r) => r.name === selectedDatacenter);
			return region?.url || cloudEnv().VITE_APP_API_URL;
		})
		.with("engine", () => getConfig().apiUrl)
		.otherwise(() => {
			throw new Error("Not in a valid context");
		});

	const codeSnippet = `import { registry } from "./registry";

// Automatically reads token from env
registry.startRunner();`;
	return (
		<div className="space-y-8">
			<p>
				Runners run actors as long-running background processes without
				exposing an HTTP endpoint. Read more about <b>Runtime Modes</b>{" "}
				in the
				<a
					href={docsLinks.runtimeModes}
					target="_blank"
					rel="noopener noreferrer"
					className="ml-1 underline"
				>
					documentation
				</a>
				.
			</p>
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
				<EnvVariables endpoint={endpoint} showRunnerName={false} />
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
							<a
								href={docsLinks.quickstart.backend}
								target="_blank"
								rel="noopener noreferrer"
							>
								<span className="cursor-pointer hover:underline">
									See JavaScript Documentation{" "}
									<Icon
										icon={faChevronRight}
										className="text-xs"
									/>
								</span>
							</a>
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
	);
}

export function CloudApiTokens() {
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
		<div className="pb-4 pb-8 px-6 max-w-5xl mx-auto my-8 border-b @6xl:border @6xl:rounded-lg">
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
