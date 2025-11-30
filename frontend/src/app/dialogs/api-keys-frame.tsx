import { faKey, faTrash, Icon } from "@rivet-gg/icons";
import { useMutation, useQuery } from "@tanstack/react-query";
import { useRouteContext } from "@tanstack/react-router";
import {
	Button,
	type DialogContentProps,
	Frame,
	Skeleton,
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from "@/components";
import { queryClient } from "@/queries/global";
import { useDialog } from "@/app/use-dialog";

interface ApiKeysFrameContentProps extends DialogContentProps {}

export default function ApiKeysFrameContent({
	onClose,
}: ApiKeysFrameContentProps) {
	const { dataProvider } = useRouteContext({
		from: "/_context/_cloud/orgs/$organization/projects/$project",
	});

	const { data, isLoading } = useQuery(dataProvider.apiKeysQueryOptions());

	const { open: openCreateApiKey, dialog: createApiKeyDialog } =
		useDialog.CreateApiKey({});

	return (
		<>
			<Frame.Header>
				<Frame.Title className="gap-2 flex items-center">
					<Icon icon={faKey} />
					<div>API Keys</div>
				</Frame.Title>
				<Frame.Description>
					API keys provide programmatic access to this project. Keep
					them secure and never share them publicly.
				</Frame.Description>
			</Frame.Header>
			<Frame.Content>
				{isLoading ? (
					<div className="space-y-2">
						<Skeleton className="w-full h-12" />
						<Skeleton className="w-full h-12" />
						<Skeleton className="w-full h-12" />
					</div>
				) : data?.apiKeys.length === 0 ? (
					<div className="text-center py-8 text-muted-foreground">
						No API keys yet. Create one to get started.
					</div>
				) : (
					<Table>
						<TableHeader>
							<TableRow>
								<TableHead>Name</TableHead>
								<TableHead>Key</TableHead>
								<TableHead>Created</TableHead>
								<TableHead>Expires</TableHead>
								<TableHead w="min" />
							</TableRow>
						</TableHeader>
						<TableBody>
							{data?.apiKeys.map((apiKey) => (
								<ApiKeyRow
									key={apiKey.id}
									apiKey={apiKey}
									dataProvider={dataProvider}
								/>
							))}
						</TableBody>
					</Table>
				)}
			</Frame.Content>
			<Frame.Footer>
				<Button variant="default" onClick={() => openCreateApiKey()}>
					Create API Key
				</Button>
				<Button variant="secondary" onClick={onClose}>
					Close
				</Button>
			</Frame.Footer>
			{createApiKeyDialog}
		</>
	);
}

interface ApiKeyRowProps {
	apiKey: {
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

function ApiKeyRow({ apiKey, dataProvider }: ApiKeyRowProps) {
	const { mutate: revoke, isPending } = useMutation(
		dataProvider.revokeApiKeyMutationOptions({
			onSuccess: async () => {
				await queryClient.invalidateQueries(
					dataProvider.apiKeysQueryOptions(),
				);
			},
		}),
	);

	const createdDate = new Date(apiKey.createdAt).toLocaleDateString();
	const expiresDate = apiKey.expiresAt
		? new Date(apiKey.expiresAt).toLocaleDateString()
		: "Never";

	return (
		<TableRow className={apiKey.revoked ? "opacity-50" : ""}>
			<TableCell>{apiKey.name}</TableCell>
			<TableCell>
				<code className="text-xs bg-muted px-2 py-1 rounded">
					cloud_api_...{apiKey.lastFourChars}
				</code>
			</TableCell>
			<TableCell>{createdDate}</TableCell>
			<TableCell>{expiresDate}</TableCell>
			<TableCell>
				{!apiKey.revoked && (
					<Button
						variant="destructive"
						size="sm"
						isLoading={isPending}
						onClick={() => {
							revoke({ apiKeyId: apiKey.id });
						}}
					>
						<Icon icon={faTrash} className="mr-1" />
						Revoke
					</Button>
				)}
				{apiKey.revoked && (
					<span className="text-muted-foreground text-sm">Revoked</span>
				)}
			</TableCell>
		</TableRow>
	);
}
