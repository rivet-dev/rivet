import { faTrash, Icon } from "@rivet-gg/icons";
import { useState } from "react";
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
} from "@/components";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select";
import { authClient } from "@/lib/auth";

interface OrgMembersFrameContentProps extends DialogContentProps {}

type Role = "member" | "admin" | "owner";

export default function OrgMembersFrameContent({
	onClose,
}: OrgMembersFrameContentProps) {
	const { data: org, isPending } = authClient.useActiveOrganization();
	const { data: session } = authClient.useSession();

	const [inviteEmail, setInviteEmail] = useState("");
	const [inviteRole, setInviteRole] = useState<Role>("member");
	const [inviteError, setInviteError] = useState<string | null>(null);
	const [invitePending, setInvitePending] = useState(false);

	const handleInvite = async () => {
		if (!org || !inviteEmail.trim()) return;
		setInviteError(null);
		setInvitePending(true);

		const result = await authClient.organization.inviteMember({
			email: inviteEmail.trim(),
			role: inviteRole,
			organizationId: org.id,
		});

		setInvitePending(false);

		if (result.error) {
			setInviteError(result.error.message ?? "Failed to send invitation");
			return;
		}

		setInviteEmail("");
	};

	const handleRemoveMember = async (userId: string) => {
		if (!org) return;
		await authClient.organization.removeMember({
			memberIdOrEmail: userId,
			organizationId: org.id,
		});
	};

	const handleCancelInvitation = async (invitationId: string) => {
		await authClient.organization.cancelInvitation({ invitationId });
	};

	return (
		<>
			<Frame.Header>
				<Frame.Title>Manage Members</Frame.Title>
				<Frame.Description>
					View members and invite people to your organization.
				</Frame.Description>
			</Frame.Header>
			<Frame.Content className="space-y-6 max-h-[60vh] overflow-y-auto">
				{isPending ? (
					<div className="space-y-2">
						<Skeleton className="w-full h-10" />
						<Skeleton className="w-full h-10" />
						<Skeleton className="w-full h-10" />
					</div>
				) : (
					<>
						<Table>
							<TableHeader>
								<TableRow>
									<TableHead>Member</TableHead>
									<TableHead>Role</TableHead>
									<TableHead className="w-min" />
								</TableRow>
							</TableHeader>
							<TableBody>
								{org?.members.length === 0 ? (
									<TableRow>
										<TableCell
											colSpan={3}
											className="text-center py-8 text-muted-foreground"
										>
											No members yet.
										</TableCell>
									</TableRow>
								) : (
									org?.members.map((member) => {
										const user = (
											member as unknown as {
												user: {
													id: string;
													name: string;
													email: string;
													image?: string | null;
												};
											}
										).user;
										return (
											<TableRow key={member.id}>
												<TableCell>
													<div className="flex items-center gap-2">
														<Avatar className="size-6">
															<AvatarImage
																src={
																	user?.image ??
																	undefined
																}
															/>
															<AvatarFallback>
																{(
																	user?.name ??
																	user?.email ??
																	"?"
																)[0].toUpperCase()}
															</AvatarFallback>
														</Avatar>
														<div className="text-sm">
															<p className="font-medium">
																{user?.name}
															</p>
															<p className="text-muted-foreground">
																{user?.email}
															</p>
														</div>
													</div>
												</TableCell>
												<TableCell>
													<Badge variant="secondary">
														{member.role}
													</Badge>
												</TableCell>
												<TableCell>
													{member.userId !==
														session?.user.id && (
														<Button
															variant="ghost"
															size="icon"
															onClick={() =>
																handleRemoveMember(
																	member.userId,
																)
															}
														>
															<Icon
																icon={faTrash}
																className="size-4 text-destructive"
															/>
														</Button>
													)}
												</TableCell>
											</TableRow>
										);
									})
								)}
							</TableBody>
						</Table>

						{(org?.invitations?.length ?? 0) > 0 && (
							<div className="space-y-2">
								<p className="text-sm font-medium text-muted-foreground">
									Pending invitations
								</p>
								<Table>
									<TableHeader>
										<TableRow>
											<TableHead>Email</TableHead>
											<TableHead>Role</TableHead>
											<TableHead className="w-min" />
										</TableRow>
									</TableHeader>
									<TableBody>
										{org?.invitations.map((inv) => (
											<TableRow key={inv.id}>
												<TableCell className="text-sm">
													{inv.email}
												</TableCell>
												<TableCell>
													<Badge variant="outline">
														{inv.role}
													</Badge>
												</TableCell>
												<TableCell>
													<Button
														variant="ghost"
														size="sm"
														onClick={() =>
															handleCancelInvitation(
																inv.id,
															)
														}
													>
														Revoke
													</Button>
												</TableCell>
											</TableRow>
										))}
									</TableBody>
								</Table>
							</div>
						)}

						<div className="space-y-3 pt-2 border-t">
							<p className="text-sm font-medium">Invite a member</p>
							<div className="flex gap-2">
								<div className="flex-1">
									<Label
										htmlFor="invite-email"
										className="sr-only"
									>
										Email address
									</Label>
									<Input
										id="invite-email"
										type="email"
										placeholder="colleague@company.com"
										value={inviteEmail}
										onChange={(e) =>
											setInviteEmail(e.target.value)
										}
									/>
								</div>
								<Select
									value={inviteRole}
									onValueChange={(v) =>
										setInviteRole(v as Role)
									}
								>
									<SelectTrigger className="w-28">
										<SelectValue />
									</SelectTrigger>
									<SelectContent>
										<SelectItem value="member">
											Member
										</SelectItem>
										<SelectItem value="admin">
											Admin
										</SelectItem>
										<SelectItem value="owner">
											Owner
										</SelectItem>
									</SelectContent>
								</Select>
								<Button
									onClick={handleInvite}
									isLoading={invitePending}
									disabled={!inviteEmail.trim()}
								>
									Invite
								</Button>
							</div>
							{inviteError && (
								<p className="text-sm text-destructive">
									{inviteError}
								</p>
							)}
						</div>
					</>
				)}
			</Frame.Content>
			<Frame.Footer>
				<Button variant="secondary" onClick={onClose}>
					Close
				</Button>
			</Frame.Footer>
		</>
	);
}
