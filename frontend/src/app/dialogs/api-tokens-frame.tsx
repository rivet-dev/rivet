import { faPlus, faQuestionCircle, faTrash, Icon } from "@rivet-gg/icons";
import { useMutation, useQuery } from "@tanstack/react-query";
import { HelpDropdown } from "@/app/help-dropdown";
import { useDialog } from "@/app/use-dialog";
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
import { useCloudNamespaceDataProvider } from "@/components/actors";
import { queryClient } from "@/queries/global";

interface ApiTokensFrameContentProps extends DialogContentProps {}

export default function ApiTokensFrameContent({
	onClose,
}: ApiTokensFrameContentProps) {
	const dataProvider = useCloudNamespaceDataProvider();

	const { data, isLoading } = useQuery(dataProvider.apiTokensQueryOptions());

	const { open: openCreateApiToken, dialog: createApiTokenDialog } =
		useDialog.CreateApiToken({});

	return (
		<>
			<Frame.Header>
				<Frame.Title className="gap-2 flex items-center">
					<div>Cloud API Tokens</div>
					<HelpDropdown>
						<Button variant="ghost" size="icon">
							<Icon icon={faQuestionCircle} />
						</Button>
					</HelpDropdown>
				</Frame.Title>
				<Frame.Description>
					Cloud API tokens provide programmatic access to the Rivet
					Cloud API. Keep them secure and never share them publicly.
				</Frame.Description>
			</Frame.Header>
			<Frame.Content>
				{isLoading ? (
					<div className="space-y-2">
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
								<TableHead className="w-min" />
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
									/>
								))
							)}
							<TableRow>
								<TableCell colSpan={5}>
									<Button
										variant="outline"
										onClick={() => openCreateApiToken()}
										startIcon={<Icon icon={faPlus} />}
										className="w-full"
									>
										Create Cloud API Token
									</Button>
								</TableCell>
							</TableRow>
						</TableBody>
					</Table>
				)}
			</Frame.Content>
			<Frame.Footer>
				<Button variant="secondary" onClick={onClose}>
					Close
				</Button>
			</Frame.Footer>
			{createApiTokenDialog}
		</>
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
}

function ApiTokenRow({ apiToken }: ApiTokenRowProps) {
	const dataProvider = useCloudNamespaceDataProvider();
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
