import {
	faEllipsisVertical,
	faPaperPlaneTop,
	faTrash,
	faUserPlus,
	Icon,
} from "@rivet-gg/icons";
import { useMutation } from "@tanstack/react-query";
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
	Skeleton,
	toast,
	WithTooltip,
} from "@/components";
import { queryClient } from "@/queries/global";
import { authClient } from "@/lib/auth";
import { SettingsCard } from "./settings-card";

export function MembersPanel() {
	const [showInvite, setShowInvite] = useState(false);
	const { data: org, isPending } = authClient.useActiveOrganization();
	const { data: session } = authClient.useSession();

	const viewerRole =
		org?.members.find((m) => m.userId === session?.user?.id)?.role ??
		"member";
	const canManageMembers = viewerRole === "owner" || viewerRole === "admin";
	const canManageInvitations = canManageMembers;

	const { mutateAsync: inviteMember } = useMutation({
		mutationFn: async (email: string) => {
			if (!org) return;
			const result = await authClient.organization.inviteMember({
				email,
				role: "member",
				organizationId: org.id,
			});
			if (result.error) throw result.error;
		},
		onSuccess: () => toast.success("Invitation sent."),
	});

	if (!org && !isPending) {
		return (
			<div className="flex h-64 items-center justify-center rounded-lg border border-dashed border-border">
				<p className="text-sm text-muted-foreground">
					No active organization.
				</p>
			</div>
		);
	}

	return (
		<div className="space-y-4">
			{canManageInvitations ? (
				<div className="flex justify-end">
					<Button
						variant="outline"
						size="sm"
						startIcon={
							<Icon icon={faUserPlus} className="size-3.5" />
						}
						onClick={() => setShowInvite((v) => !v)}
					>
						Invite member
					</Button>
				</div>
			) : null}

			<AnimatePresence initial={false}>
				{showInvite && canManageInvitations ? (
					<motion.div
						key="invite-form"
						initial={{ height: 0, opacity: 0, marginTop: 0 }}
						animate={{ height: "auto", opacity: 1, marginTop: 16 }}
						exit={{ height: 0, opacity: 0, marginTop: 0 }}
						transition={{
							height: { duration: 0.22, ease: "easeInOut" },
							marginTop: { duration: 0.22, ease: "easeInOut" },
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
								<InviteMemberForm.RootError />
							</InviteMemberForm.Form>
						</div>
					</motion.div>
				) : null}
			</AnimatePresence>

			<SettingsCard divided>
				<div className="grid grid-cols-[2fr_2fr_28px] items-center gap-4 px-5 py-2 text-[11px] font-medium uppercase tracking-wide text-muted-foreground border-b border-foreground/10 bg-foreground/[0.02]">
					<div>Name</div>
					<div>Email</div>
					<div />
				</div>

				{isPending ? (
					<>
						<MemberRowSkeleton />
						<MemberRowSkeleton />
						<MemberRowSkeleton />
					</>
				) : null}

				{org?.members.map((member, idx) => {
					const user = member.user;
					const isYou = member.userId === session?.user.id;
					const isLast =
						idx === (org.members.length ?? 0) - 1 &&
						org.invitations.filter(
							(inv) => inv.status === "pending",
						).length === 0;
					return (
						<MemberRow
							key={member.id}
							memberId={member.id}
							userId={member.userId}
							organizationId={org.id}
							avatarUrl={user?.image ?? undefined}
							name={user?.name ?? user?.email ?? "?"}
							email={user?.email ?? ""}
							role={member.role ?? "member"}
							isYou={isYou}
							canManage={canManageMembers}
							viewerRole={viewerRole}
							last={isLast}
						/>
					);
				})}

				{org?.invitations
					.filter((inv) => inv.status === "pending")
					.map((inv, idx, arr) => (
						<InvitationRow
							key={inv.id}
							email={inv.email}
							invitationId={inv.id}
							organizationId={org.id}
							canManage={canManageInvitations}
							last={idx === arr.length - 1}
						/>
					))}
			</SettingsCard>
		</div>
	);
}

function MemberRow({
	memberId,
	userId,
	organizationId,
	avatarUrl,
	name,
	email,
	role,
	isYou,
	canManage,
	viewerRole,
	last,
}: {
	memberId: string;
	userId: string;
	organizationId: string;
	avatarUrl?: string;
	name: string;
	email: string;
	role: string;
	isYou: boolean;
	canManage: boolean;
	viewerRole: string;
	last?: boolean;
}) {
	const initials = name
		.split(" ")
		.map((part) => part[0])
		.slice(0, 2)
		.join("")
		.toUpperCase();
	const isOwner = role === "owner";
	return (
		<div
			className={cn(
				"group grid grid-cols-[2fr_2fr_28px] items-center gap-4 px-5 py-3 text-sm transition-colors hover:bg-foreground/[0.025]",
				!last && "border-b border-foreground/10",
			)}
		>
			<div className="flex items-center gap-2.5 min-w-0">
				<Avatar className="size-7 shrink-0">
					<AvatarImage src={avatarUrl} />
					<AvatarFallback className="text-[10px]">
						{initials || "?"}
					</AvatarFallback>
				</Avatar>
				<div className="flex items-center gap-1.5 min-w-0">
					<span className="text-foreground truncate">{name}</span>
					{isYou ? (
						<span className="inline-flex items-center rounded-full border border-primary/20 bg-primary/10 px-2 py-0.5 text-[11px] font-medium text-primary">
							You
						</span>
					) : null}
				</div>
			</div>
			<div className="text-muted-foreground truncate">{email}</div>
			<div className="flex items-center justify-end gap-2">
				{isOwner ? (
					<span className="shrink-0 inline-flex items-center rounded-full border border-foreground/15 bg-foreground/[0.06] px-2 py-0.5 text-[11px] font-medium text-foreground">
						Owner
					</span>
				) : null}
				<div className="opacity-0 transition-opacity group-hover:opacity-100 focus-within:opacity-100">
					{canManage ? (
						<MemberRowMenu
							memberId={memberId}
							userId={userId}
							organizationId={organizationId}
							role={role}
							isSelf={isYou}
							viewerRole={viewerRole}
						/>
					) : null}
				</div>
			</div>
		</div>
	);
}

function MemberRowMenu({
	memberId,
	userId,
	organizationId,
	role,
	isSelf,
	viewerRole,
}: {
	memberId: string;
	userId: string;
	organizationId: string;
	role: string;
	isSelf: boolean;
	viewerRole: string;
}) {
	const invalidate = () =>
		queryClient.invalidateQueries({ queryKey: ["organizations"] });

	const { mutate: setRole, isPending: isUpdatingRole } = useMutation({
		mutationFn: async (nextRole: "owner" | "admin" | "member") => {
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
	const viewerIsOwner = viewerRole === "owner";
	const disabled = isUpdatingRole || isRemoving;

	if (isSelf) {
		return (
			<Button
				variant="ghost"
				size="icon-sm"
				type="button"
				aria-label="Member actions"
				aria-disabled
				title="You can't change your own role"
				onClick={(e) => e.preventDefault()}
				onPointerDown={(e) => e.preventDefault()}
				className="cursor-not-allowed opacity-50 hover:bg-transparent"
			>
				<Icon icon={faEllipsisVertical} />
			</Button>
		);
	}

	return (
		<DropdownMenu>
			<DropdownMenuTrigger asChild>
				<Button
					variant="ghost"
					size="icon-sm"
					aria-label="Member actions"
					disabled={disabled}
				>
					<Icon icon={faEllipsisVertical} />
				</Button>
			</DropdownMenuTrigger>
			<DropdownMenuContent align="end" className="min-w-[10rem]">
				{viewerIsOwner && !isOwner ? (
					<DropdownMenuItem onSelect={() => setRole("owner")}>
						Promote to owner
					</DropdownMenuItem>
				) : null}
				{!isAdmin && !isOwner ? (
					<DropdownMenuItem onSelect={() => setRole("admin")}>
						Promote to admin
					</DropdownMenuItem>
				) : null}
				{isAdmin ? (
					<DropdownMenuItem onSelect={() => setRole("member")}>
						Demote to member
					</DropdownMenuItem>
				) : null}
				{!isOwner ? (
					<>
						<DropdownMenuSeparator />
						<DropdownMenuItem
							onSelect={() => remove()}
							className="text-destructive focus:text-destructive"
						>
							Remove from organization
						</DropdownMenuItem>
					</>
				) : null}
			</DropdownMenuContent>
		</DropdownMenu>
	);
}

function InvitationRow({
	email,
	invitationId,
	organizationId,
	canManage,
	last,
}: {
	email: string;
	invitationId: string;
	organizationId: string;
	canManage: boolean;
	last?: boolean;
}) {
	const { mutate: resend, isPending: isResendPending } = useMutation({
		mutationFn: async () => {
			const result = await authClient.organization.inviteMember({
				email,
				role: "member",
				organizationId,
				resend: true,
			});
			if (result.error) throw result.error;
		},
		onSuccess: () => toast.success("Invitation resent."),
	});
	const { mutate: revoke, isPending: isRevokePending } = useMutation({
		mutationFn: async () => {
			const result = await authClient.organization.cancelInvitation({
				invitationId,
			});
			if (result.error) throw result.error;
		},
		onSuccess: () => toast.success("Invitation revoked."),
	});

	const initials = email[0]?.toUpperCase() ?? "?";

	return (
		<div
			className={cn(
				"group grid grid-cols-[2fr_2fr_28px] items-center gap-4 px-5 py-3 text-sm transition-colors hover:bg-foreground/[0.025]",
				!last && "border-b border-foreground/10",
			)}
		>
			<div className="flex items-center gap-2.5 min-w-0">
				<Avatar className="size-7 shrink-0">
					<AvatarFallback className="text-[10px]">
						{initials}
					</AvatarFallback>
				</Avatar>
				<div className="flex items-center gap-1.5 min-w-0">
					<span className="text-muted-foreground italic">
						Invited
					</span>
					<span className="inline-flex items-center rounded-full border border-foreground/10 bg-foreground/[0.03] px-2 py-0.5 text-[11px] font-medium text-muted-foreground">
						Pending
					</span>
				</div>
			</div>
			<div className="text-muted-foreground truncate">{email}</div>
			<div className="flex items-center gap-1 justify-end opacity-0 transition-opacity group-hover:opacity-100 focus-within:opacity-100">
				{canManage ? (
					<>
						<WithTooltip
							content="Resend"
							delayDuration={0}
							trigger={
								<Button
									variant="ghost"
									size="icon-sm"
									isLoading={isResendPending}
									onClick={() => resend()}
								>
									<Icon
										icon={faPaperPlaneTop}
										className="size-3"
									/>
								</Button>
							}
						/>
						<WithTooltip
							content="Revoke"
							delayDuration={0}
							trigger={
								<Button
									variant="ghost"
									size="icon-sm"
									isLoading={isRevokePending}
									onClick={() => revoke()}
								>
									<Icon
										icon={faTrash}
										className="size-3 text-destructive"
									/>
								</Button>
							}
						/>
					</>
				) : null}
			</div>
		</div>
	);
}

function MemberRowSkeleton() {
	return (
		<div className="grid grid-cols-[2fr_2fr_28px] items-center gap-4 px-5 py-3 border-b border-foreground/10">
			<div className="flex items-center gap-2.5">
				<Skeleton className="size-7 rounded-full" />
				<Skeleton className="h-4 w-32" />
			</div>
			<Skeleton className="h-4 w-40" />
			<div />
		</div>
	);
}
