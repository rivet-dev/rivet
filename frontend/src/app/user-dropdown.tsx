import { faChevronDown, faPlus, Icon } from "@rivet-gg/icons";
import { useQuery } from "@tanstack/react-query";
import { useMatchRoute, useNavigate, useParams } from "@tanstack/react-router";
import {
	Avatar,
	AvatarFallback,
	AvatarImage,
	Button,
	DropdownMenu,
	DropdownMenuCheckboxItem,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuPortal,
	DropdownMenuSeparator,
	DropdownMenuSub,
	DropdownMenuSubContent,
	DropdownMenuSubTrigger,
	DropdownMenuTrigger,
	Skeleton,
} from "@/components";
import { useCloudDataProvider } from "@/components/actors";
import { authClient } from "@/lib/auth";

export function UserDropdown({ children }: { children?: React.ReactNode }) {
	const params = useParams({
		strict: false,
	});

	const { data: session } = authClient.useSession();
	const navigate = useNavigate();
	const match = useMatchRoute();

	const isMatchingProjectRoute = match({
		to: "/orgs/$organization/projects/$project",
		fuzzy: true,
	});

	return (
		<DropdownMenu>
			<DropdownMenuTrigger asChild={!params.organization || !!children}>
				{children ||
					(params.organization ? (
						<Preview org={params.organization} />
					) : (
						<Button
							variant="ghost"
							size="xs"
							className="text-muted-foreground justify-between py-1 min-h-8 gap-2 w-full"
							endIcon={<Icon icon={faChevronDown} />}
						>
							{session?.user?.email}
						</Button>
					))}
			</DropdownMenuTrigger>
			<DropdownMenuContent>
				{isMatchingProjectRoute ? (
					<>
						<DropdownMenuItem
							onSelect={() => {
								return navigate({
									to: ".",
									search: (old) => ({
										...old,
										modal: "billing",
									}),
								});
							}}
						>
							Billing
						</DropdownMenuItem>
						<DropdownMenuSeparator />
					</>
				) : null}
				{params.organization ? (
					<DropdownMenuSub>
						<DropdownMenuSubTrigger>
							Switch Organization
						</DropdownMenuSubTrigger>
						<DropdownMenuPortal>
							<DropdownMenuSubContent>
								<OrganizationSwitcher
									value={params.organization}
								/>
							</DropdownMenuSubContent>
						</DropdownMenuPortal>
					</DropdownMenuSub>
				) : null}
				<DropdownMenuItem
					onSelect={() => {
						authClient.signOut();
					}}
				>
					Logout
				</DropdownMenuItem>
			</DropdownMenuContent>
		</DropdownMenu>
	);
}

function Preview({ org }: { org: string }) {
	const { isLoading, data } = useQuery(
		useCloudDataProvider().organizationQueryOptions({ org }),
	);

	return (
		<Button
			variant="ghost"
			size="xs"
			className="text-muted-foreground justify-between py-1 min-h-8 gap-2 w-full"
			endIcon={<Icon icon={faChevronDown} />}
		>
			<div className="flex gap-2 items-center w-full min-w-0">
				<Avatar className="size-5">
					<AvatarImage src={data?.logo ?? undefined} />
					<AvatarFallback>
						{isLoading ? (
							<Skeleton className="h-5 w-5" />
						) : (
							data?.name[0].toUpperCase()
						)}
					</AvatarFallback>
				</Avatar>
				<span className="text-sm truncate">
					{isLoading ? (
						<Skeleton className="w-full h-4 flex-1" />
					) : (
						data?.name
					)}
				</span>
			</div>
		</Button>
	);
}

function OrganizationSwitcher({ value }: { value: string | undefined }) {
	const { data: organizations, isPending: isLoading } =
		authClient.useListOrganizations();

	const navigate = useNavigate();

	return (
		<>
			{isLoading ? (
				<>
					<DropdownMenuCheckboxItem>
						<Skeleton className="h-4 w-full" />
					</DropdownMenuCheckboxItem>
					<DropdownMenuCheckboxItem>
						<Skeleton className="h-4 w-full" />
					</DropdownMenuCheckboxItem>
					<DropdownMenuCheckboxItem>
						<Skeleton className="h-4 w-full" />
					</DropdownMenuCheckboxItem>
				</>
			) : null}
			{organizations?.map((org) => (
				<DropdownMenuCheckboxItem
					key={org.id}
					checked={org.id === value}
					onSelect={() => {
						authClient.organization.setActive({
							organizationId: org.id,
						});
						navigate({
							to: `/orgs/$organization`,
							params: {
								organization: org.id,
							},
						});
					}}
				>
					<Avatar className="size-4 mr-2">
						<AvatarImage src={org.logo ?? undefined} />
						<AvatarFallback>
							{org.name[0].toUpperCase()}
						</AvatarFallback>
					</Avatar>
					{org.name}
				</DropdownMenuCheckboxItem>
			))}
			<DropdownMenuItem
				onSelect={() => {
					navigate({
						to: ".",
						search: (old) => ({
							...old,
							modal: "create-organization",
						}),
					});
				}}
				indicator={<Icon icon={faPlus} className="size-4" />}
			>
				Create a new organization
			</DropdownMenuItem>
		</>
	);
}
