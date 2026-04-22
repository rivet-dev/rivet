import {
	faChevronDown,
	faCheck,
	faEllipsisVertical,
	faUserPlus,
	Icon,
} from "@rivet-gg/icons";
import { useState } from "react";
import {
	Button,
	cn,
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuTrigger,
} from "@/components";

type AssignableRole = "Admin" | "Member";
type MemberRole = "Owner" | AssignableRole;

interface Member {
	name: string;
	email: string;
	role: MemberRole;
	joinedAt: string;
	color: string;
}

const INITIAL_MEMBERS: Member[] = [
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

function OwnerBadge() {
	return (
		<span className="inline-flex items-center rounded-full border border-primary/20 bg-primary/10 px-2 py-0.5 text-[11px] font-medium text-primary">
			Owner
		</span>
	);
}

function RoleSelect({
	value,
	onChange,
}: {
	value: AssignableRole;
	onChange: (role: AssignableRole) => void;
}) {
	const roles: AssignableRole[] = ["Admin", "Member"];
	return (
		<DropdownMenu>
			<DropdownMenuTrigger asChild>
				<Button
					variant="outline"
					size="sm"
					className="h-7 px-2.5 gap-1.5 text-xs font-normal"
				>
					{value}
					<Icon icon={faChevronDown} className="w-2.5" />
				</Button>
			</DropdownMenuTrigger>
			<DropdownMenuContent align="start" className="w-36">
				{roles.map((role) => (
					<DropdownMenuItem
						key={role}
						onSelect={() => onChange(role)}
						className="text-xs"
					>
						<span className="flex-1">{role}</span>
						{role === value ? (
							<Icon
								icon={faCheck}
								className="w-3 text-muted-foreground"
							/>
						) : null}
					</DropdownMenuItem>
				))}
			</DropdownMenuContent>
		</DropdownMenu>
	);
}

function MemberRowMenu({ onRemove }: { onRemove: () => void }) {
	return (
		<DropdownMenu>
			<DropdownMenuTrigger asChild>
				<button
					type="button"
					className="text-muted-foreground hover:text-foreground rounded p-1 -m-1"
					aria-label="Member options"
				>
					<Icon icon={faEllipsisVertical} className="w-3.5" />
				</button>
			</DropdownMenuTrigger>
			<DropdownMenuContent align="end" className="w-40">
				<DropdownMenuItem
					onSelect={onRemove}
					className="text-xs text-destructive focus:text-destructive focus:bg-destructive/10"
				>
					Remove member
				</DropdownMenuItem>
			</DropdownMenuContent>
		</DropdownMenu>
	);
}

export function MembersContent() {
	const [members, setMembers] = useState<Member[]>(INITIAL_MEMBERS);

	const updateRole = (email: string, role: AssignableRole) => {
		setMembers((prev) =>
			prev.map((m) => (m.email === email ? { ...m, role } : m)),
		);
	};

	const removeMember = (email: string) => {
		setMembers((prev) => prev.filter((m) => m.email !== email));
	};

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
					{members.map((member) => (
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
								{member.role === "Owner" ? (
									<OwnerBadge />
								) : (
									<RoleSelect
										value={member.role}
										onChange={(role) =>
											updateRole(member.email, role)
										}
									/>
								)}
							</span>
							{member.role === "Owner" ? (
								<span className="w-6" />
							) : (
								<MemberRowMenu
									onRemove={() => removeMember(member.email)}
								/>
							)}
						</div>
					))}
				</div>
			</div>
		</div>
	);
}
