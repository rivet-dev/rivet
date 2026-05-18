import { faGear, faPlus, faUsers, Icon } from "@rivet-gg/icons";
import { useInfiniteQuery } from "@tanstack/react-query";
import { Link, useNavigate } from "@tanstack/react-router";
import {
	Avatar,
	AvatarFallback,
	AvatarImage,
	Button,
	cn,
	H1,
	RelativeTime,
	ScrollArea,
	SmallText,
} from "@/components";
import { useCloudDataProvider } from "@/components/actors";
import { authClient } from "@/lib/auth";
import {
	getRecentTimestamp,
	RECENT_PROJECTS_KEY,
} from "@/lib/recently-visited";
import { LazyBillingPlanBadge } from "./billing/billing-plan-badge";

export function OrgLanding({ organization }: { organization: string }) {
	const navigate = useNavigate();
	const dataProvider = useCloudDataProvider();
	const { data: projects = [], isLoading } = useInfiniteQuery(
		dataProvider.currentOrgProjectsQueryOptions(),
	);
	const { data: org } = authClient.useActiveOrganization();
	const { data: session } = authClient.useSession();

	const sorted = [...projects].sort((a, b) => {
		const aTime = getRecentTimestamp(RECENT_PROJECTS_KEY, a.name);
		const bTime = getRecentTimestamp(RECENT_PROJECTS_KEY, b.name);
		if (aTime !== bTime) return bTime - aTime;
		return a.displayName.localeCompare(b.displayName);
	});

	const heading = org?.name ? `${org.name} Projects` : "Projects";

	return (
		<div className="flex flex-1 min-h-0 my-2 mr-2 overflow-hidden rounded-xl border border-foreground/10 bg-card">
			<ScrollArea className="h-full w-full">
				<div className="px-6 py-6 max-w-6xl mx-auto space-y-8">
					<header className="flex items-start justify-between gap-4">
						<div>
							<H1 className="text-2xl">{heading}</H1>
							<SmallText className="text-muted-foreground mt-1">
								Each row is a project in the{" "}
								{org?.name ?? "this"} organization.
							</SmallText>
						</div>
						{sorted.length > 0 ? (
							<Button
								variant="outline"
								size="sm"
								startIcon={<Icon icon={faPlus} />}
								onClick={() => {
									navigate({
										to: ".",
										search: (old) => ({
											...(old as Record<string, unknown>),
											modal: "create-project",
											organization,
										}),
									});
								}}
							>
								Create Project
							</Button>
						) : null}
					</header>

					{!isLoading && sorted.length === 0 ? (
						<div className="flex flex-col items-center gap-3 rounded-md border border-dashed bg-card/50 px-6 py-10 text-center">
							<H1 className="text-base">No projects yet</H1>
							<SmallText className="text-muted-foreground max-w-md">
								Create a project to start deploying actors in this
								organization.
							</SmallText>
							<Button
								variant="default"
								size="sm"
								startIcon={<Icon icon={faPlus} />}
								onClick={() => {
									navigate({
										to: ".",
										search: (old) => ({
											...(old as Record<string, unknown>),
											modal: "create-project",
											organization,
										}),
									});
								}}
							>
								Create Project
							</Button>
						</div>
					) : (
						<div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 gap-3">
							{sorted.map((project) => (
								<Link
									key={project.id}
									to="/orgs/$organization/projects/$project"
									params={{
										organization,
										project: project.name,
									}}
									className={cn(
										"group relative flex flex-col items-start gap-2 rounded-lg border border-foreground/10 bg-foreground/[0.02] p-4 text-left transition-colors",
										"hover:border-foreground/20 hover:bg-foreground/[0.05]",
										"focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
										"min-h-[130px] cursor-pointer",
									)}
								>
									<div className="absolute top-3 right-3">
										<LazyBillingPlanBadge
											project={project.name}
											organization={organization}
										/>
									</div>
									<div className="font-medium text-sm leading-tight truncate pr-14 w-full">
										{project.displayName}
									</div>
									<SmallText className="text-muted-foreground text-xs leading-tight font-mono-console truncate w-full">
										{project.name}
									</SmallText>
									{project.createdAt ? (
										<SmallText className="text-muted-foreground text-[11px] mt-auto pt-1">
											Created{" "}
											<RelativeTime
												time={new Date(project.createdAt)}
											/>
										</SmallText>
									) : null}
								</Link>
							))}
						</div>
					)}

					<MembersSection
						organization={organization}
						members={org?.members ?? []}
						currentUserId={session?.user?.id}
					/>
				</div>
			</ScrollArea>
		</div>
	);
}

function MembersSection({
	organization,
	members,
	currentUserId,
}: {
	organization: string;
	members: Array<{
		id: string;
		userId: string;
		role?: string;
		user?: { name?: string | null; email?: string | null; image?: string | null };
	}>;
	currentUserId: string | undefined;
}) {
	const navigate = useNavigate();
	const visible = members.slice(0, 5);
	const remaining = Math.max(0, members.length - visible.length);

	const openMembers = () => {
		navigate({
			to: ".",
			search: (old) => ({
				...(old as Record<string, unknown>),
				modal: "org-members",
				organization,
			}),
		});
	};

	return (
		<section>
			<header className="flex items-start justify-between gap-4 mb-3">
				<div>
					<h2 className="text-base font-semibold text-foreground flex items-center gap-2">
						<Icon
							icon={faUsers}
							className="size-3.5 text-muted-foreground"
						/>
						Members
					</h2>
					<SmallText className="text-muted-foreground mt-0.5">
						People with access to this organization.
					</SmallText>
				</div>
				<Button
					variant="outline"
					size="sm"
					startIcon={<Icon icon={faGear} />}
					onClick={openMembers}
				>
					Manage members
				</Button>
			</header>
			<div className="rounded-md border border-foreground/10 bg-card overflow-hidden">
				{visible.length === 0 ? (
					<div className="px-3 py-4 text-center">
						<SmallText className="text-muted-foreground">
							No members yet.
						</SmallText>
					</div>
				) : (
					visible.map((member) => {
						const user = member.user;
						const initial =
							(user?.name ?? user?.email ?? "?")[0]?.toUpperCase() ??
							"?";
						return (
							<div
								key={member.id}
								className="grid grid-cols-[1fr_120px] gap-4 items-center px-3 py-2.5 text-xs border-b border-foreground/10 last:border-b-0"
							>
								<div className="flex items-center gap-2 min-w-0">
									<Avatar className="size-6 shrink-0">
										<AvatarImage src={user?.image ?? undefined} />
										<AvatarFallback>{initial}</AvatarFallback>
									</Avatar>
									<div className="min-w-0">
										<div className="flex items-center gap-1.5">
											<span className="font-medium text-foreground truncate">
												{user?.name ?? user?.email ?? "Unknown"}
											</span>
											{member.userId === currentUserId ? (
												<span className="inline-flex items-center rounded-full border border-primary/20 bg-primary/10 px-1.5 py-0 text-[10px] font-medium text-primary">
													You
												</span>
											) : null}
										</div>
										{user?.email && user.email !== user.name ? (
											<div className="text-muted-foreground truncate text-[11px]">
												{user.email}
											</div>
										) : null}
									</div>
								</div>
								<div className="text-muted-foreground capitalize">
									{member.role ?? "member"}
								</div>
							</div>
						);
					})
				)}
				{remaining > 0 ? (
					<button
						type="button"
						onClick={openMembers}
						className="w-full px-3 py-2.5 text-xs text-left text-muted-foreground hover:bg-foreground/[0.04] transition-colors border-t border-foreground/10"
					>
						+{remaining} more
					</button>
				) : null}
			</div>
		</section>
	);
}
