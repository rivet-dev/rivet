import {
	Avatar,
	AvatarFallback,
	AvatarImage,
	SmallText,
} from "@/components";
import { authClient } from "@/lib/auth";
import { orgConicGradient, paletteForLetter } from "@/lib/org-palette";
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

			{/* TODO: Re-enable once deletion propagates to the org switcher properly. */}
			{/* <DangerZone
				organizationId={org.id}
				organizationName={org.name ?? ""}
				isOwner={isOwner}
			/> */}
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

