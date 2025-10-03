import { useClerk, useOrganizationList } from "@clerk/clerk-react";
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
import { VisibilitySensor } from "@/components/visibility-sensor";

export function UserDropdown() {
	const params = useParams({
		strict: false,
	});

	const clerk = useClerk();
	const navigate = useNavigate();
	const match = useMatchRoute();

	const isMatchingProjectRoute = match({
		to: "/orgs/$organization/projects/$project",
	});

	return (
		<DropdownMenu>
			<DropdownMenuTrigger asChild={!params.organization}>
				{params.organization ? (
					<Preview org={params.organization} />
				) : (
					<Button
						variant="ghost"
						size="xs"
						className="text-muted-foreground justify-between py-1 min-h-8 gap-2 w-full"
						endIcon={<Icon icon={faChevronDown} />}
					>
						{clerk.user?.primaryEmailAddress?.emailAddress}
					</Button>
				)}
			</DropdownMenuTrigger>
			<DropdownMenuContent>
				<DropdownMenuItem
					onSelect={() => {
						clerk.openUserProfile();
					}}
				>
					Profile
				</DropdownMenuItem>
				{clerk.organization ? (
					<DropdownMenuItem
						onSelect={() => {
							clerk.openOrganizationProfile();
						}}
					>
						Settings
					</DropdownMenuItem>
				) : null}
				{clerk.organization ? (
					<DropdownMenuItem
						onSelect={() => {
							clerk.openOrganizationProfile({
								__experimental_startPath:
									"/organization-members",
							});
						}}
					>
						Members
					</DropdownMenuItem>
				) : null}
				{isMatchingProjectRoute ? (
					<DropdownMenuItem
						onSelect={() => {
							navigate({
								to: ".",
								search: (old) => ({ ...old, modal: "billing" }),
							});
						}}
					>
						Billing
					</DropdownMenuItem>
				) : null}
				<DropdownMenuSeparator />
				{clerk.organization ? (
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
						clerk.signOut();
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
					<AvatarImage src={data?.imageUrl} />
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
	const {
		userMemberships: {
			data: userMemberships = [],
			isLoading,
			hasNextPage,
			fetchNext,
		},
	} = useOrganizationList({
		userMemberships: {
			infinite: true,
		},
	});

	const clerk = useClerk();
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
					<DropdownMenuCheckboxItem>
						<Skeleton className="h-4 w-full" />
					</DropdownMenuCheckboxItem>
					<DropdownMenuCheckboxItem>
						<Skeleton className="h-4 w-full" />
					</DropdownMenuCheckboxItem>
				</>
			) : null}
			{userMemberships.map((membership) => (
				<DropdownMenuCheckboxItem
					key={membership.id}
					checked={membership.organization.id === value}
					onSelect={() => {
						clerk.setActive({
							organization: membership.organization.id,
							navigate: () => {
								navigate({
									to: `/orgs/$organization`,
									params: {
										organization:
											membership.organization.id,
									},
								});
							},
						});
					}}
				>
					<Avatar className="size-4 mr-2">
						<AvatarImage src={membership.organization.imageUrl} />
						<AvatarFallback>
							{membership.organization.name[0].toUpperCase()}
						</AvatarFallback>
					</Avatar>
					{membership.organization.name}
				</DropdownMenuCheckboxItem>
			))}
			<DropdownMenuItem
				onSelect={() => {
					clerk.openCreateOrganization({ hideSlug: true });
				}}
				indicator={<Icon icon={faPlus} className="size-4" />}
			>
				Create a new organization
			</DropdownMenuItem>
			{hasNextPage ? <VisibilitySensor onChange={fetchNext} /> : null}
		</>
	);
}
