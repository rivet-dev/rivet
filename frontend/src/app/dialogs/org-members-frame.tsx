import { faPaperPlaneTop, faTrash, Icon } from "@rivet-gg/icons";
import { useMutation } from "@tanstack/react-query";
import { toast } from "sonner";
import * as InviteMemberForm from "@/app/forms/invite-member-form";
import {
	Avatar,
	AvatarFallback,
	AvatarImage,
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
	WithTooltip,
} from "@/components";
import { Badge } from "@/components/ui/badge";
import { authClient } from "@/lib/auth";

function RemoveMemberButton({
	organizationId,
	userId,
}: {
	organizationId: string;
	userId: string;
}) {
	const { mutate, isPending } = useMutation({
		mutationFn: async () => {
			const result = await authClient.organization.removeMember({
				memberIdOrEmail: userId,
				organizationId,
			});
			if (result.error) throw result.error;
		},
		onSuccess: () => toast.success("Member removed."),
	});

	return (
		<WithTooltip
			content="Remove member"
			delayDuration={0}
			trigger={
				<Button
					variant="ghost"
					size="icon"
					isLoading={isPending}
					onClick={() => mutate()}
				>
					<Icon icon={faTrash} className="size-4 text-destructive" />
				</Button>
			}
		/>
	);
}

function InvitationActions({
	organizationId,
	invitationId,
	email,
}: {
	organizationId: string;
	invitationId: string;
	email: string;
}) {
	const { mutate: resend, isPending: isResendPending } = useMutation({
		mutationFn: async () => {
			const result = await authClient.organization.inviteMember({
				email,
				role: "owner",
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

	return (
		<div className="flex gap-1">
			<WithTooltip
				content="Resend"
				delayDuration={0}
				trigger={
					<Button
						variant="ghost"
						size="icon"
						isLoading={isResendPending}
						onClick={() => resend()}
					>
						<Icon icon={faPaperPlaneTop} />
					</Button>
				}
			/>
			<WithTooltip
				content="Revoke"
				delayDuration={0}
				trigger={
					<Button
						variant="ghost"
						size="icon"
						isLoading={isRevokePending}
						onClick={() => revoke()}
					>
						<Icon icon={faTrash} className="text-destructive" />
					</Button>
				}
			/>
		</div>
	);
}

interface OrgMembersFrameContentProps extends DialogContentProps {}

export default function OrgMembersFrameContent(_: OrgMembersFrameContentProps) {
	const { data: org, isPending } = authClient.useActiveOrganization();
	const { data: session } = authClient.useSession();

	const { mutateAsync: inviteMember } = useMutation({
		mutationFn: async (email: string) => {
			if (!org) return;
			const result = await authClient.organization.inviteMember({
				email,
				role: "owner",
				organizationId: org.id,
			});
			if (result.error) throw result.error;
		},
		onSuccess: () => toast.success("Invitation sent."),
	});

	return (
		<>
			<Frame.Header>
				<Frame.Title>Manage Members</Frame.Title>
				<Frame.Description>
					View members and invite people to your organization.
				</Frame.Description>
			</Frame.Header>
			<Frame.Content className="space-y-4">
				{isPending ? (
					<div className="space-y-2">
						<Skeleton className="w-full h-10" />
						<Skeleton className="w-full h-10" />
						<Skeleton className="w-full h-10" />
					</div>
				) : (
					<>
						<div className="max-h-[35vh] overflow-y-auto">
							<Table>
								<TableHeader>
									<TableRow>
										<TableHead className="w-full">
											Member
										</TableHead>
										<TableHead className="w-min" />
									</TableRow>
								</TableHeader>
								<TableBody>
									{org?.members.map((member) => {
										const user = member.user;
										return (
											<TableRow key={member.id}>
												<TableCell className="w-full">
													<div className="flex items-center gap-2">
														<Avatar className="size-6">
															<AvatarImage
																src={
																	user?.image ??
																	undefined
																}
															/>
															<AvatarFallback>
																{(user?.name ??
																	user?.email ??
																	"?")[0].toUpperCase()}
															</AvatarFallback>
														</Avatar>
														<div className="text-sm">
															<div className="flex items-center gap-1.5">
																<p className="font-medium">
																	{user?.name}
																</p>
																{member.userId ===
																	session
																		?.user
																		.id && (
																	<Badge
																		variant="outline"
																		className="text-xs py-0"
																	>
																		You
																	</Badge>
																)}
															</div>
															<p className="text-muted-foreground">
																{user?.email}
															</p>
														</div>
													</div>
												</TableCell>
												<TableCell>
													<div className="flex justify-end">
														{member.userId !==
															session?.user.id &&
															org && (
																<RemoveMemberButton
																	organizationId={
																		org.id
																	}
																	userId={
																		member.userId
																	}
																/>
															)}
													</div>
												</TableCell>
											</TableRow>
										);
									})}
									{org?.invitations
										.filter(
											(inv) => inv.status === "pending",
										)
										.map((inv) => (
											<TableRow key={inv.id}>
												<TableCell className="w-full">
													<div className="flex items-center gap-2">
														<Avatar className="size-6">
															<AvatarFallback>
																{inv.email[0].toUpperCase()}
															</AvatarFallback>
														</Avatar>
														<div className="text-sm">
															<p className="text-muted-foreground">
																{inv.email}
															</p>
															<p className="text-xs text-muted-foreground">
																Invitation sent
															</p>
														</div>
													</div>
												</TableCell>
												<TableCell>
													{org && (
														<InvitationActions
															organizationId={
																org.id
															}
															invitationId={
																inv.id
															}
															email={inv.email}
														/>
													)}
												</TableCell>
											</TableRow>
										))}
									{!org?.members.length &&
										!org?.invitations.filter(
											(inv) => inv.status === "pending",
										).length && (
											<TableRow>
												<TableCell
													colSpan={2}
													className="text-center py-8 text-muted-foreground"
												>
													No members yet.
												</TableCell>
											</TableRow>
										)}
								</TableBody>
							</Table>
						</div>

						<div className="space-y-3">
							<p className="text-sm font-medium mb-2 mt-4">
								Invite a member
							</p>
							<InviteMemberForm.Form
								defaultValues={{ email: "" }}
								onSubmit={async ({ email }, form) => {
									try {
										await inviteMember(email);
										form.reset();
									} catch {
										form.setError("root", {
											message:
												"Failed to send invitation.",
										});
									}
								}}
							>
								<div className="flex gap-2">
									<div className="flex-1">
										<InviteMemberForm.EmailField />
									</div>
									<InviteMemberForm.Submit>
										Invite
									</InviteMemberForm.Submit>
								</div>
								<InviteMemberForm.RootError />
							</InviteMemberForm.Form>
						</div>
					</>
				)}
			</Frame.Content>
		</>
	);
}
