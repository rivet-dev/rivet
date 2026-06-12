import { faChevronDown, Icon } from "@rivet-gg/icons";
import { Link } from "@tanstack/react-router";
import { Button, cn } from "@/components";
import { authClient } from "@/lib/auth";
import { features } from "@/lib/features";
import { orgConicGradient, paletteForLetter } from "@/lib/org-palette";
import { ContextSwitcher } from "./context-switcher";
import { FeedbackButton } from "./feedback-button";
import { HelpButton } from "./help-button";
import { Logo } from "./logo";
import { UserDropdown } from "./user-dropdown";

export function TopBar() {
	return (
		<header
			className={cn(
				"z-20 flex items-center gap-3 h-11 px-3 mt-2 mr-2",
				"bg-card border border-border rounded-lg shrink-0",
			)}
		>
			<TopBarLogo />
			<ContextSwitcher inline />
			<div className="ml-auto flex items-center gap-1">
				<FeedbackButton />
				{features.support ? <HelpButton /> : null}
				<DocsButton />
				{features.auth ? (
					<>
						<div className="mx-1 h-5 w-px bg-border" />
						<UserDropdownTrigger />
					</>
				) : null}
			</div>
		</header>
	);
}

function TopBarLogo() {
	return (
		<Link to="/" className="flex items-center gap-2 shrink-0">
			<Logo className="h-5 w-auto text-foreground" />
		</Link>
	);
}

function DocsButton() {
	return (
		<Button
			variant="ghost"
			size="sm"
			className="text-muted-foreground hover:text-foreground"
			asChild
		>
			<a
				href="https://www.rivet.dev/docs"
				target="_blank"
				rel="noopener noreferrer"
			>
				Docs
			</a>
		</Button>
	);
}

function UserDropdownTrigger() {
	return (
		<UserDropdown>
			<Button
				variant="ghost"
				size="sm"
				className="gap-2 text-muted-foreground hover:text-foreground"
				endIcon={
					<Icon
						icon={faChevronDown}
						className="size-2.5 opacity-60"
					/>
				}
			>
				<OrgIdentity />
			</Button>
		</UserDropdown>
	);
}

function OrgIdentity() {
	const { data: org, isPending } = authClient.useActiveOrganization();
	const { data: session } = authClient.useSession();

	// Fall back to the user's name if no active org (e.g. brand-new account).
	const name =
		org?.name ??
		session?.user?.name ??
		session?.user?.email?.split("@")[0] ??
		"Account";
	const logo = org?.logo ?? undefined;
	const initial = name[0]?.toUpperCase() ?? "?";

	return (
		<>
			<span className="size-5 rounded-full overflow-hidden flex items-center justify-center shrink-0">
				{logo ? (
					// biome-ignore lint/performance/noImgElement: small avatar, no Next runtime
					<img src={logo} alt="" className="size-full object-cover" />
				) : (
					<span
						className="size-full flex items-center justify-center text-[10px] font-semibold text-white"
						style={{
							backgroundImage: orgConicGradient(
								paletteForLetter(name),
							),
						}}
					>
						{initial}
					</span>
				)}
			</span>
			<span className="text-sm">{isPending && !org ? "…" : name}</span>
		</>
	);
}
