import { useClerk, useOrganizationList } from "@clerk/clerk-react";
import { faChevronRight, faPlus, Icon } from "@rivet-gg/icons";
import { useQuery } from "@tanstack/react-query";
import { createFileRoute, Link } from "@tanstack/react-router";
import { Logo } from "@/app/logo";
import {
	Avatar,
	AvatarFallback,
	AvatarImage,
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
	Skeleton,
} from "@/components";
import { VisibilitySensor } from "@/components/visibility-sensor";

export const Route = createFileRoute("/_context/_cloud/orgs/")({
	component: RouteComponent,
});

function RouteComponent() {
	return (
		<div className="flex flex-col gap-6 px-4 w-full mx-auto h-screen min-h-0 max-h-screen items-center justify-safe-center overflow-auto py-8">
			<div className="flex flex-col items-center gap-6 min-h-fit">
				<Logo className="h-10 mb-4" />
				<Card className="w-full sm:w-96 mb-4">
					<CardHeader>
						<CardTitle>Organizations</CardTitle>
						<CardDescription>
							Select or create an organization to get started.
						</CardDescription>
					</CardHeader>
					<CardContent>
						<OrganizationList />
					</CardContent>
				</Card>
			</div>
		</div>
	);
}

function OrganizationList() {
	const {
		userMemberships: { data = [], isLoading, hasNextPage, fetchNext },
	} = useOrganizationList({
		userMemberships: {
			infinite: true,
		},
	});

	const clerk = useClerk();
	const navigate = Route.useNavigate();

	return (
		<div className="flex flex-col border rounded-md w-full">
			{isLoading
				? Array(5)
						.fill(null)
						.map((_, i) => <ListItemSkeleton key={i} />)
				: null}

			{data.map((membership) => (
				<button
					type="button"
					onClick={() => {
						clerk.setActive({
							organization: membership.organization.id,
							navigate: () => {
								navigate({
									to: "/orgs/$organization",
									params: {
										organization:
											membership.organization.id,
									},
								});
							},
						});
					}}
					key={membership.id}
					className="p-2 border-b last:border-0 w-full flex text-left items-center hover:bg-accent rounded-md transition-colors"
				>
					<Avatar className="size-4 mr-2">
						<AvatarImage src={membership.organization.imageUrl} />
						<AvatarFallback>
							{membership.organization.name[0].toUpperCase()}
						</AvatarFallback>
					</Avatar>
					<span className="flex-1 truncate">
						{membership.organization.name}
					</span>
					<Icon icon={faChevronRight} className="ml-auto" />
				</button>
			))}
			{hasNextPage ? <VisibilitySensor onChange={fetchNext} /> : null}
			<button
				onClick={() => clerk.openCreateOrganization()}
				type="button"
			>
				<div className="p-2 w-full flex items-center justify-center text-sm hover:bg-accent rounded-md transition-colors cursor-pointer">
					<Icon icon={faPlus} className="mr-1" /> Create Organization
				</div>
			</button>
		</div>
	);
}

function ListItemSkeleton() {
	return (
		<div className="p-2 border-b last:border-0 w-full flex text-left items-center rounded-md transition-colors h-10">
			<Skeleton className="size-4 mr-2 rounded-full" />
			<Skeleton className="flex-1 h-4 rounded" />
		</div>
	);
}
