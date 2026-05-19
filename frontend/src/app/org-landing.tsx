import {
	faEllipsis,
	faFolder,
	faPlus,
	faUserPlus,
	faUsers,
	Icon,
} from "@rivet-gg/icons";
import { useInfiniteQuery, useMutation } from "@tanstack/react-query";
import { Link, useNavigate } from "@tanstack/react-router";
import { AnimatePresence, motion } from "framer-motion";
import { useState } from "react";
import * as InviteMemberForm from "@/app/forms/invite-member-form";
import {
	Avatar,
	AvatarFallback,
	AvatarImage,
	Button,
	cn,
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuSeparator,
	DropdownMenuTrigger,
	H1,
	RelativeTime,
	ScrollArea,
	SmallText,
	toast,
} from "@/components";
import { queryClient } from "@/queries/global";
import { useCloudDataProvider } from "@/components/actors";
import { authClient } from "@/lib/auth";
import { orgConicGradient, paletteForLetter } from "@/lib/org-palette";
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

	return (
		<div className="flex flex-1 min-h-0 my-2 mr-2 overflow-hidden rounded-xl border border-foreground/10 bg-card">
			<ScrollArea className="h-full w-full">
				<div className="px-6 py-6 max-w-6xl mx-auto space-y-8">
					<header className="flex items-center gap-3">
						<Avatar className="size-10 shrink-0">
							<AvatarImage src={org?.logo ?? undefined} />
							<AvatarFallback
								className="text-white text-base font-semibold"
								style={{
									backgroundImage: orgConicGradient(
										paletteForLetter(org?.name ?? ""),
									),
								}}
							>
								{(org?.name?.[0] ?? "?").toUpperCase()}
							</AvatarFallback>
						</Avatar>
						<H1 className="text-2xl">
							{org?.name ?? "Organization"}
						</H1>
					</header>

					<section>
						<header className="flex items-start justify-between gap-4 mb-3">
							<div>
								<h2 className="text-base font-semibold text-foreground flex items-center gap-2">
									<Icon
										icon={faFolder}
										className="size-3.5 text-muted-foreground"
									/>
									Projects
								</h2>
								<SmallText className="text-muted-foreground mt-0.5">
									Each row is a project in this organization.
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
												...(old as Record<
													string,
													unknown
												>),
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
					</section>

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
	const [showInvite, setShowInvite] = useState(false);
	const visible = members.slice(0, 5);
	const remaining = Math.max(0, members.length - visible.length);

	const { data: org } = authClient.useActiveOrganization();
	const organizationId = org?.id;

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

	const { mutateAsync: inviteMember } = useMutation({
		mutationFn: async (email: string) => {
			if (!organizationId) throw new Error("No active organization");
			const result = await authClient.organization.inviteMember({
				email,
				role: "member",
				organizationId,
			});
			if (result.error) throw result.error;
		},
		onSuccess: () => {
			toast.success("Invitation sent.");
			void queryClient.invalidateQueries({
				queryKey: ["organizations"],
			});
		},
	});

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
					startIcon={<Icon icon={faUserPlus} />}
					onClick={() => setShowInvite((v) => !v)}
				>
					Invite member
				</Button>
			</header>
			<AnimatePresence initial={false}>
				{showInvite ? (
					<motion.div
						key="invite-form"
						initial={{ height: 0, opacity: 0, marginBottom: 0 }}
						animate={{
							height: "auto",
							opacity: 1,
							marginBottom: 12,
						}}
						exit={{ height: 0, opacity: 0, marginBottom: 0 }}
						transition={{
							height: { duration: 0.22, ease: "easeInOut" },
							marginBottom: { duration: 0.22, ease: "easeInOut" },
							opacity: { duration: 0.15, ease: "easeInOut" },
						}}
						className="overflow-hidden"
					>
						<div className="rounded-lg border border-foreground/10 bg-card p-3">
							<InviteMemberForm.Form
								defaultValues={{ email: "" }}
								mode="onSubmit"
								revalidateMode="onSubmit"
								onSubmit={async ({ email }, form) => {
									try {
										await inviteMember(email);
										form.reset();
										setShowInvite(false);
									} catch {
										form.setError("root", {
											message: "Failed to send invitation.",
										});
									}
								}}
							>
								<div className="grid grid-cols-[1fr_auto] gap-2 items-start">
									<InviteMemberForm.EmailField />
									<InviteMemberForm.Submit>
										Invite
									</InviteMemberForm.Submit>
								</div>
							</InviteMemberForm.Form>
						</div>
					</motion.div>
				) : null}
			</AnimatePresence>
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
						const isSelf = member.userId === currentUserId;
						return (
							<div
								key={member.id}
								className="grid grid-cols-[1fr_120px_auto] gap-4 items-center px-3 py-2.5 text-xs border-b border-foreground/10 last:border-b-0"
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
											{isSelf ? (
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
								<MemberRowMenu
									memberId={member.id}
									userId={member.userId}
									role={member.role ?? "member"}
									isSelf={isSelf}
								/>
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

function MemberRowMenu({
	memberId,
	userId,
	role,
	isSelf,
}: {
	memberId: string;
	userId: string;
	role: string;
	isSelf: boolean;
}) {
	const { data: org } = authClient.useActiveOrganization();
	const organizationId = org?.id;

	const invalidate = () =>
		queryClient.invalidateQueries({ queryKey: ["organizations"] });

	const { mutate: setRole, isPending: isUpdatingRole } = useMutation({
		mutationFn: async (nextRole: "owner" | "admin" | "member") => {
			if (!organizationId) throw new Error("No active organization");
			const result = await authClient.organization.updateMemberRole({
				memberId,
				role: nextRole,
				organizationId,
			});
			if (result.error) throw result.error;
		},
		onSuccess: (_data, nextRole) => {
			toast.success(`Member set to ${nextRole}.`);
			void invalidate();
		},
		onError: () => toast.error("Couldn't update member role."),
	});

	const { mutate: remove, isPending: isRemoving } = useMutation({
		mutationFn: async () => {
			if (!organizationId) throw new Error("No active organization");
			const result = await authClient.organization.removeMember({
				memberIdOrEmail: userId,
				organizationId,
			});
			if (result.error) throw result.error;
		},
		onSuccess: () => {
			toast.success("Member removed.");
			void invalidate();
		},
		onError: () => toast.error("Couldn't remove member."),
	});

	const isOwner = role === "owner";
	const isAdmin = role === "admin";
	const disabled = isUpdatingRole || isRemoving;

	return (
		<DropdownMenu>
			<DropdownMenuTrigger asChild>
				<Button
					variant="ghost"
					size="icon-sm"
					aria-label="Member actions"
					disabled={disabled || isSelf}
					title={isSelf ? "You can't change your own role" : undefined}
					className="disabled:pointer-events-auto disabled:cursor-not-allowed disabled:hover:bg-transparent"
				>
					<Icon icon={faEllipsis} />
				</Button>
			</DropdownMenuTrigger>
			<DropdownMenuContent align="end" className="min-w-[10rem]">
				{!isOwner ? (
					<DropdownMenuItem onSelect={() => setRole("owner")}>
						Promote to owner
					</DropdownMenuItem>
				) : null}
				{!isAdmin && !isOwner ? (
					<DropdownMenuItem onSelect={() => setRole("admin")}>
						Promote to admin
					</DropdownMenuItem>
				) : null}
				{isOwner || isAdmin ? (
					<DropdownMenuItem onSelect={() => setRole("member")}>
						Demote to member
					</DropdownMenuItem>
				) : null}
				<DropdownMenuSeparator />
				<DropdownMenuItem
					onSelect={() => remove()}
					className="text-destructive focus:text-destructive"
				>
					Remove from organization
				</DropdownMenuItem>
			</DropdownMenuContent>
		</DropdownMenu>
	);
}
