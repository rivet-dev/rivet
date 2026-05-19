import { faPaperPlaneTop, faTrash, faUserPlus, Icon } from "@rivet-gg/icons";
import { useMutation } from "@tanstack/react-query";
import { useState } from "react";
import * as InviteMemberForm from "@/app/forms/invite-member-form";
import {
	Avatar,
	AvatarFallback,
	AvatarImage,
	Button,
	cn,
	Skeleton,
	toast,
	WithTooltip,
} from "@/components";
import { authClient } from "@/lib/auth";

export function MembersPanel() {
	const [showInvite, setShowInvite] = useState(false);
	const { data: org, isPending } = authClient.useActiveOrganization();
	const { data: session } = authClient.useSession();

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
			<div className="flex justify-end">
				<Button
					variant="outline"
					size="sm"
					startIcon={<Icon icon={faUserPlus} className="size-3.5" />}
					onClick={() => setShowInvite((v) => !v)}
				>
					Invite member
				</Button>
			</div>

			{showInvite ? (
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
			) : null}

			<div className="rounded-lg border border-foreground/10 bg-card overflow-hidden">
				<div className="grid grid-cols-[2fr_2fr_1fr_auto] items-center gap-4 px-4 py-2.5 text-[11px] font-medium uppercase tracking-wider text-muted-foreground border-b border-foreground/10">
					<div>Name</div>
					<div>Email</div>
					<div>Role</div>
					<div className="w-7" />
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
							avatarUrl={user?.image ?? undefined}
							name={user?.name ?? user?.email ?? "?"}
							email={user?.email ?? ""}
							role={
								member.role.charAt(0).toUpperCase() +
								member.role.slice(1)
							}
							isOwner={member.role === "owner"}
							isYou={isYou}
							last={isLast}
							removable={!isYou && org !== null}
							onRemove={async () => {
								if (!org) return;
								try {
									await authClient.organization.removeMember({
										memberIdOrEmail: member.userId,
										organizationId: org.id,
									});
									toast.success("Member removed.");
								} catch {
									toast.error("Failed to remove member.");
								}
							}}
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
							last={idx === arr.length - 1}
						/>
					))}
			</div>
		</div>
	);
}

function MemberRow({
	avatarUrl,
	name,
	email,
	role,
	isOwner,
	isYou,
	last,
	removable,
	onRemove,
}: {
	avatarUrl?: string;
	name: string;
	email: string;
	role: string;
	isOwner: boolean;
	isYou: boolean;
	last?: boolean;
	removable: boolean;
	onRemove: () => Promise<void>;
}) {
	const [isRemoving, setIsRemoving] = useState(false);
	const initials = name
		.split(" ")
		.map((part) => part[0])
		.slice(0, 2)
		.join("")
		.toUpperCase();
	return (
		<div
			className={cn(
				"grid grid-cols-[2fr_2fr_1fr_auto] items-center gap-4 px-4 py-3 text-sm",
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
			<div>
				<span className="text-muted-foreground capitalize">
					{isOwner ? "Owner" : role}
				</span>
			</div>
			<div className="w-7 flex justify-end">
				{removable ? (
					<WithTooltip
						content="Remove member"
						delayDuration={0}
						trigger={
							<Button
								variant="ghost"
								size="icon-sm"
								isLoading={isRemoving}
								onClick={async () => {
									setIsRemoving(true);
									try {
										await onRemove();
									} finally {
										setIsRemoving(false);
									}
								}}
							>
								<Icon
									icon={faTrash}
									className="size-3.5 text-destructive"
								/>
							</Button>
						}
					/>
				) : null}
			</div>
		</div>
	);
}

function InvitationRow({
	email,
	invitationId,
	organizationId,
	last,
}: {
	email: string;
	invitationId: string;
	organizationId: string;
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
				"grid grid-cols-[2fr_2fr_1fr_auto] items-center gap-4 px-4 py-3 text-sm",
				!last && "border-b border-foreground/10",
			)}
		>
			<div className="flex items-center gap-2.5 min-w-0">
				<Avatar className="size-7 shrink-0">
					<AvatarFallback className="text-[10px]">
						{initials}
					</AvatarFallback>
				</Avatar>
				<span className="text-muted-foreground italic">Invited</span>
			</div>
			<div className="text-muted-foreground truncate">{email}</div>
			<div>
				<span className="text-muted-foreground text-xs">
					Pending invite
				</span>
			</div>
			<div className="flex items-center gap-1 justify-end">
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
							<Icon icon={faPaperPlaneTop} className="size-3" />
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
			</div>
		</div>
	);
}

function MemberRowSkeleton() {
	return (
		<div className="grid grid-cols-[2fr_2fr_1fr_auto] items-center gap-4 px-4 py-3 border-b border-foreground/10">
			<div className="flex items-center gap-2.5">
				<Skeleton className="size-7 rounded-full" />
				<Skeleton className="h-4 w-32" />
			</div>
			<Skeleton className="h-4 w-40" />
			<Skeleton className="h-4 w-16" />
			<div />
		</div>
	);
}
