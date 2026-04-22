import {
	faEllipsisVertical,
	faUserPlus,
	Icon,
} from "@rivet-gg/icons";
import { Button, cn } from "@/components";

interface Member {
	name: string;
	email: string;
	role: "Owner" | "Admin" | "Member";
	joinedAt: string;
	color: string;
}

const MEMBERS: Member[] = [
	{
		name: "Nicholas Kissel",
		email: "nicholas@rivet.dev",
		role: "Owner",
		joinedAt: "Jan 12, 2025",
		color: "bg-blue-500",
	},
	{
		name: "Nathan Flurry",
		email: "nathan@rivet.dev",
		role: "Admin",
		joinedAt: "Jan 12, 2025",
		color: "bg-emerald-500",
	},
	{
		name: "Forest Anderson",
		email: "forest@rivet.dev",
		role: "Admin",
		joinedAt: "Feb 3, 2025",
		color: "bg-amber-500",
	},
	{
		name: "Ava Chen",
		email: "ava@rivet.dev",
		role: "Member",
		joinedAt: "Mar 18, 2025",
		color: "bg-rose-500",
	},
];

function Avatar({ name, color }: { name: string; color: string }) {
	const initials = name
		.split(" ")
		.map((part) => part[0])
		.slice(0, 2)
		.join("")
		.toUpperCase();
	return (
		<div
			className={cn(
				"flex size-7 items-center justify-center rounded-full text-[11px] font-medium text-white",
				color,
			)}
		>
			{initials}
		</div>
	);
}

function RoleBadge({ role }: { role: Member["role"] }) {
	const variants: Record<Member["role"], string> = {
		Owner: "bg-primary/10 text-primary border-primary/20",
		Admin: "bg-muted-foreground/10 text-foreground border-border",
		Member: "bg-muted-foreground/5 text-muted-foreground border-border",
	};
	return (
		<span
			className={cn(
				"inline-flex items-center rounded-full border px-2 py-0.5 text-[11px] font-medium",
				variants[role],
			)}
		>
			{role}
		</span>
	);
}

export function MembersContent() {
	return (
		<div className="space-y-6 pb-10">
			<div className="flex items-center justify-end">
				<Button
					variant="outline"
					size="sm"
					className="gap-1.5 shrink-0"
				>
					<Icon icon={faUserPlus} className="w-3" />
					Invite member
				</Button>
			</div>
			<div className="rounded-xl border dark:border-white/10 bg-card overflow-hidden">
				<div>
					<div
						className={cn(
							"grid grid-cols-[2fr_1.5fr_1fr_auto] items-center gap-4 px-6 py-3",
							"text-[11px] font-medium uppercase tracking-wider text-muted-foreground",
							"border-b dark:border-white/10 bg-muted/20",
						)}
					>
						<span>Name</span>
						<span>Email</span>
						<span>Role</span>
						<span className="w-6" />
					</div>
					{MEMBERS.map((member) => (
						<div
							key={member.email}
							className="grid grid-cols-[2fr_1.5fr_1fr_auto] items-center gap-4 px-6 py-3.5 text-sm border-b dark:border-white/10 last:border-b-0 hover:bg-muted/20 transition-colors"
						>
							<div className="flex items-center gap-3 min-w-0">
								<Avatar
									name={member.name}
									color={member.color}
								/>
								<div className="min-w-0">
									<div className="text-foreground font-medium truncate">
										{member.name}
									</div>
									<div className="text-[11px] text-muted-foreground">
										Joined {member.joinedAt}
									</div>
								</div>
							</div>
							<span className="text-muted-foreground truncate">
								{member.email}
							</span>
							<span>
								<RoleBadge role={member.role} />
							</span>
							<button
								type="button"
								className="text-muted-foreground hover:text-foreground rounded p-1 -m-1"
								aria-label="Options"
							>
								<Icon
									icon={faEllipsisVertical}
									className="w-3.5"
								/>
							</button>
						</div>
					))}
				</div>
			</div>
		</div>
	);
}
