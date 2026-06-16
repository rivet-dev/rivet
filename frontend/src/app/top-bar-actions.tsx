import { faBook, Icon } from "@rivet-gg/icons";
import { type ComponentPropsWithoutRef, forwardRef } from "react";
import { Avatar, AvatarFallback, AvatarImage, Button } from "@/components";
import { authClient } from "@/lib/auth";
import { features } from "@/lib/features";
import { orgConicGradient, paletteForLetter } from "@/lib/org-palette";
import { FeedbackButton } from "./feedback-button";
import { HelpButton } from "./help-button";
import { UserDropdown } from "./user-dropdown";

// Right-side cluster of the top bar. Shared between the dashboard `TopBar` and
// the onboarding `SidebarlessHeader` so the two stay in sync.
export function TopBarActions() {
	return (
		<div className="ml-auto flex items-center gap-1">
			<FeedbackButton />
			{features.support ? <HelpButton /> : null}
			<DocsButton />
			{features.auth ? (
				<>
					<div className="mx-1 h-5 w-px bg-border" />
					<UserDropdown>
						<UserAvatarTrigger />
					</UserDropdown>
				</>
			) : null}
		</div>
	);
}

function DocsButton() {
	return (
		<Button
			variant="ghost"
			size="sm"
			className="text-muted-foreground hover:text-foreground"
			startIcon={<Icon icon={faBook} className="size-4" />}
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

// Account avatar. The organization now lives in the breadcrumb, so the
// right-side trigger shows the signed-in user rather than the active org.
// Forwards ref/props so `DropdownMenuTrigger asChild` (Radix Slot) can wire its
// click handler and ref onto the underlying button. Without this the dropdown
// never opens.
const UserAvatarTrigger = forwardRef<
	HTMLButtonElement,
	ComponentPropsWithoutRef<typeof Button>
>((props, ref) => {
	const { data: session } = authClient.useSession();

	const name =
		session?.user?.name ?? session?.user?.email?.split("@")[0] ?? "Account";
	const image = session?.user?.image ?? undefined;
	const initial = name[0]?.toUpperCase() ?? "?";

	return (
		<Button
			ref={ref}
			variant="ghost"
			size="icon-sm"
			className="rounded-full"
			aria-label="Account menu"
			{...props}
		>
			<Avatar className="size-6">
				<AvatarImage src={image} />
				<AvatarFallback
					className="text-[10px] font-semibold text-white"
					style={{
						backgroundImage: orgConicGradient(
							paletteForLetter(name),
						),
					}}
				>
					{initial}
				</AvatarFallback>
			</Avatar>
		</Button>
	);
});
UserAvatarTrigger.displayName = "UserAvatarTrigger";
