import { useClerk, useOrganizationList } from "@clerk/clerk-react";
import { faChevronDown, faPlus, Icon } from "@rivet-gg/icons";
import { useQuery } from "@tanstack/react-query";
import { useNavigate, useParams } from "@tanstack/react-router";
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
	const org = useParams({
		strict: false,
		select: (p) => p.organization,
	});

	const clerk = useClerk();

	const { data: url } = useQuery(
		useCloudDataProvider().billingCustomerPortalSessionQueryOptions(),
	);

	return (
		<DropdownMenu>
			<DropdownMenuTrigger asChild={!org}>
				{org ? <Preview org={org} /> : null}
			</DropdownMenuTrigger>
			<DropdownMenuContent>
				<DropdownMenuItem
					onSelect={() => {
						clerk.openOrganizationProfile();
					}}
				>
					Settings
				</DropdownMenuItem>
				<DropdownMenuItem
					onSelect={() => {
						clerk.openOrganizationProfile({
							__experimental_startPath: "/organization-members",
						});
					}}
				>
					Members
				</DropdownMenuItem>
				<DropdownMenuItem
					onSelect={() => {
						window.open(url, "_blank");
					}}
				>
					Billing
				</DropdownMenuItem>
				<DropdownMenuSeparator />
				<DropdownMenuSub>
					<DropdownMenuSubTrigger>
						Switch Organization
					</DropdownMenuSubTrigger>
					<DropdownMenuPortal>
						<DropdownMenuSubContent>
							<OrganizationSwitcher value={org} />
						</DropdownMenuSubContent>
					</DropdownMenuPortal>
				</DropdownMenuSub>
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
			<div className="flex gap-2 items-center w-full">
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
				{isLoading ? (
					<Skeleton className="w-full h-4 flex-1" />
				) : (
					data?.name
				)}
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
					clerk.openCreateOrganization();
				}}
				indicator={<Icon icon={faPlus} className="size-4" />}
			>
				Create a new organization
			</DropdownMenuItem>
			{hasNextPage ? <VisibilitySensor onChange={fetchNext} /> : null}
		</>
	);
}
