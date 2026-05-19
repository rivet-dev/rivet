import {
	faRightFromBracket,
	faTrash,
	faTriangleExclamation,
	Icon,
} from "@rivet-gg/icons";
import { useMutation } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import {
	Avatar,
	AvatarFallback,
	AvatarImage,
	Button,
	SmallText,
	toast,
} from "@/components";
import { authClient } from "@/lib/auth";
import { orgConicGradient, paletteForLetter } from "@/lib/org-palette";
import { queryClient } from "@/queries/global";
import { MembersPanel } from "./members-panel";

export function OrganizationPanel() {
	const { data: org, isPending } = authClient.useActiveOrganization();
	const { data: session } = authClient.useSession();

	if (isPending && !org) {
		return (
			<div className="space-y-6">
				<div className="h-32 rounded-lg border border-foreground/10 bg-card animate-pulse" />
				<div className="h-24 rounded-lg border border-foreground/10 bg-card animate-pulse" />
			</div>
		);
	}

	if (!org) {
		return (
			<div className="flex h-48 items-center justify-center rounded-lg border border-dashed border-border">
				<SmallText className="text-muted-foreground">
					No active organization.
				</SmallText>
			</div>
		);
	}

	const members = org.members ?? [];
	const memberCount = members.length;
	const pendingInvites = (org.invitations ?? []).filter(
		(inv: { status?: string }) => inv?.status === "pending",
	).length;
	const currentMember = members.find((m) => m.userId === session?.user?.id);
	const isOwner = currentMember?.role === "owner";

	const initial = (org.name?.[0] ?? "?").toUpperCase();

	return (
		<div className="space-y-6">
			<section className="rounded-lg border border-foreground/10 bg-card overflow-hidden">
				<div className="flex items-center gap-4 p-5">
					<Avatar className="size-14 shrink-0">
						<AvatarImage src={org.logo ?? undefined} />
						<AvatarFallback
							className="text-white text-xl font-semibold"
							style={{
								backgroundImage: orgConicGradient(
									paletteForLetter(org.name ?? ""),
								),
							}}
						>
							{initial}
						</AvatarFallback>
					</Avatar>
					<div className="min-w-0 flex-1">
						<div className="flex items-center gap-2">
							<h3 className="text-base font-semibold text-foreground truncate">
								{org.name}
							</h3>
						</div>
						<SmallText className="text-muted-foreground font-mono-console">
							{org.slug}
						</SmallText>
					</div>
				</div>
				<div className="grid grid-cols-2 gap-px bg-foreground/[0.04] border-t border-foreground/10">
					<Cell label="Members" value={memberCount.toString()} />
					<Cell
						label="Pending invites"
						value={pendingInvites.toString()}
					/>
				</div>
			</section>

			<section>
				<header className="mb-3">
					<h3 className="text-sm font-semibold text-foreground">
						Members
					</h3>
				</header>
				<MembersPanel />
			</section>

			<DangerZone
				organizationId={org.id}
				organizationName={org.name ?? ""}
				isOwner={isOwner}
			/>
		</div>
	);
}

function Cell({ label, value }: { label: string; value: string }) {
	return (
		<div className="bg-card px-5 py-3">
			<div className="text-[10px] uppercase tracking-wider text-muted-foreground">
				{label}
			</div>
			<div className="text-lg font-semibold text-foreground tabular-nums">
				{value}
			</div>
		</div>
	);
}

function DangerZone({
	organizationId,
	organizationName,
	isOwner,
}: {
	organizationId: string;
	organizationName: string;
	isOwner: boolean;
}) {
	const navigate = useNavigate();

	const invalidate = () =>
		queryClient.invalidateQueries({ queryKey: ["organizations"] });

	const { mutate: leave, isPending: isLeaving } = useMutation({
		mutationFn: async () => {
			const result = await authClient.organization.leave({
				organizationId,
			});
			if (result.error) throw result.error;
		},
		onSuccess: async () => {
			toast.success(`You left ${organizationName}.`);
			await invalidate();
			return navigate({ to: "/" });
		},
		onError: () => toast.error("Couldn't leave the organization."),
	});

	const { mutate: deleteOrg, isPending: isDeleting } = useMutation({
		mutationFn: async () => {
			const result = await authClient.organization.delete({
				organizationId,
			});
			if (result.error) throw result.error;
		},
		onSuccess: async () => {
			toast.success(`Deleted ${organizationName}.`);
			await invalidate();
			return navigate({ to: "/" });
		},
		onError: () => toast.error("Couldn't delete the organization."),
	});

	return (
		<section className="rounded-lg border border-foreground/10 bg-card overflow-hidden">
			<header className="flex items-center gap-2 px-5 py-4">
				<Icon
					icon={faTriangleExclamation}
					className="size-3.5 text-destructive"
				/>
				<h3 className="text-sm font-semibold text-foreground">
					Danger zone
				</h3>
			</header>
			<div className="border-t border-foreground/10">
				<DangerRow
					title="Leave organization"
					description={
						isOwner
							? "Owners can't leave their own organization. Transfer ownership first."
							: "Remove yourself from this organization. You'll lose access to all its projects."
					}
					actionLabel="Leave"
					icon={faRightFromBracket}
					disabled={isOwner || isLeaving}
					disabledReason={
						isOwner ? "Owners can't leave." : undefined
					}
					isLoading={isLeaving}
					onClick={() => leave()}
				/>
				<DangerRow
					title="Delete organization"
					description="Permanently delete this organization and all its data. This cannot be undone."
					actionLabel="Delete"
					icon={faTrash}
					destructive
					disabled={!isOwner || isDeleting}
					disabledReason={
						!isOwner
							? "Only the owner can delete this organization."
							: undefined
					}
					isLoading={isDeleting}
					onClick={() => deleteOrg()}
				/>
			</div>
		</section>
	);
}

function DangerRow({
	title,
	description,
	actionLabel,
	icon,
	destructive,
	disabled,
	disabledReason,
	isLoading,
	onClick,
}: {
	title: string;
	description: string;
	actionLabel: string;
	icon: typeof faTrash;
	destructive?: boolean;
	disabled?: boolean;
	disabledReason?: string;
	isLoading?: boolean;
	onClick?: () => void;
}) {
	return (
		<div className="flex items-start justify-between gap-4 px-5 py-4 border-b border-foreground/10 last:border-b-0">
			<div className="min-w-0">
				<div className="text-sm font-medium text-foreground">{title}</div>
				<SmallText className="text-muted-foreground">
					{description}
				</SmallText>
			</div>
			<Button
				variant={destructive ? "destructive-outline" : "outline"}
				size="sm"
				startIcon={<Icon icon={icon} />}
				disabled={disabled}
				isLoading={isLoading}
				title={disabled ? disabledReason : undefined}
				onClick={onClick}
			>
				{actionLabel}
			</Button>
		</div>
	);
}
