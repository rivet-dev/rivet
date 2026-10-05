import { faCheck, faMoon, faSun, Icon } from "@rivet-gg/icons";
import { useState } from "react";
import { LogoMark } from "@/app/logo";
import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import { Button } from "@/components/ui/button";
import {
	Popover,
	PopoverContent,
	PopoverTrigger,
} from "@/components/ui/popover";
import { cn } from "@/components/lib/utils";
import { orgConicGradient, paletteForLetter } from "@/lib/org-palette";
import { useTheme } from "@/lib/theme";

interface MockTopBarProps {
	organization: string;
	/** Clusters sit where projects do in the real dashboard. */
	cluster: string;
}

/**
 * Static stand-in for `@/app/top-bar`. Same height, border, logo, slash
 * breadcrumb with per-segment switchers, and right-side actions, but fed with
 * props instead of router matches and queries so the mock renders without a
 * session. Keep the class names in sync with `TopBar` / `ContextSwitcher`.
 */
export function MockTopBar({ organization, cluster }: MockTopBarProps) {
	return (
		<header
			className={cn(
				"z-20 flex h-12 shrink-0 items-center gap-2 px-3",
				"border-b border-border bg-background",
			)}
		>
			<span className="flex shrink-0 items-center pr-1">
				<LogoMark className="h-6 w-6" />
			</span>
			<div className="flex min-w-0 items-center">
				<BreadcrumbSlash />
				<Segment
					label={organization}
					switcherLabel="Organizations"
					options={[organization, "Personal"]}
					leading={
						<span
							className="size-4 shrink-0 rounded-sm"
							style={{
								backgroundColor:
									paletteForLetter(organization).accent,
							}}
						/>
					}
				/>
				<BreadcrumbSlash />
				<Segment
					label={cluster}
					switcherLabel="Clusters"
					options={[cluster, `${cluster}-eu`, "dev"]}
				/>
			</div>
			<div className="ml-auto flex items-center gap-1">
				<Button
					variant="ghost"
					size="sm"
					className="text-muted-foreground hover:text-foreground"
				>
					Feedback
				</Button>
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
				<ThemeToggleButton />
				<div className="mx-1 h-5 w-px bg-border" />
				<Button
					variant="ghost"
					size="icon-sm"
					className="rounded-full"
					aria-label="Account menu"
				>
					<Avatar className="size-6">
						<AvatarFallback
							className="text-[10px] font-semibold text-white"
							style={{
								backgroundImage: orgConicGradient(
									paletteForLetter("Nick"),
								),
							}}
						>
							N
						</AvatarFallback>
					</Avatar>
				</Button>
			</div>
		</header>
	);
}

function Segment({
	label,
	switcherLabel,
	options,
	leading,
	onSelect,
}: {
	label: string;
	switcherLabel: string;
	options: string[];
	leading?: React.ReactNode;
	onSelect?: (option: string) => void;
}) {
	const [open, setOpen] = useState(false);
	return (
		<Popover open={open} onOpenChange={setOpen}>
			<div className="flex items-center">
				<Button
					variant="ghost"
					className="flex h-auto items-center gap-2 rounded-lg px-2 py-1 text-sm font-medium text-foreground hover:bg-foreground/[0.06]"
				>
					{leading}
					<span className="truncate">{label}</span>
				</Button>
				<PopoverTrigger asChild>
					<Button
						variant="ghost"
						aria-label={`Open ${switcherLabel.toLowerCase()} switcher`}
						className="flex h-auto items-center self-stretch rounded-lg px-1.5 py-1 text-foreground hover:bg-foreground/[0.06] data-[state=open]:bg-foreground/[0.06]"
					>
						<UnfoldIcon />
					</Button>
				</PopoverTrigger>
			</div>
			<PopoverContent className="w-56 p-1" align="start">
				<p className="px-2 py-1.5 text-xs font-medium text-muted-foreground">
					{switcherLabel}
				</p>
				{options.map((option) => (
					<button
						key={option}
						type="button"
						onClick={() => {
							onSelect?.(option);
							setOpen(false);
						}}
						className="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-sm hover:bg-accent"
					>
						<span className="flex-1 truncate text-left">
							{option}
						</span>
						{option === label ? (
							<Icon
								icon={faCheck}
								className="size-3 text-primary"
							/>
						) : null}
					</button>
				))}
			</PopoverContent>
		</Popover>
	);
}

function ThemeToggleButton() {
	const { theme, toggle } = useTheme();
	const label =
		theme === "dark" ? "Switch to light mode" : "Switch to dark mode";
	return (
		<Button
			variant="ghost"
			size="icon-sm"
			aria-label={label}
			title={label}
			className="text-muted-foreground hover:text-foreground"
			onClick={toggle}
		>
			<Icon icon={theme === "dark" ? faSun : faMoon} className="size-4" />
		</Button>
	);
}

function BreadcrumbSlash() {
	return (
		<span className="mx-0.5 shrink-0 text-muted-foreground/40" aria-hidden>
			<svg
				viewBox="0 0 24 24"
				width="16"
				height="16"
				stroke="currentColor"
				strokeWidth="1"
				strokeLinecap="round"
				strokeLinejoin="round"
				fill="none"
				shapeRendering="geometricPrecision"
				role="img"
				aria-label="Breadcrumb separator"
			>
				<path d="M16 3.549L7.12 20.600" />
			</svg>
		</span>
	);
}

function UnfoldIcon() {
	return (
		<svg
			viewBox="0 0 24 24"
			width="14"
			height="14"
			stroke="currentColor"
			strokeWidth="2"
			strokeLinecap="round"
			strokeLinejoin="round"
			fill="none"
			className="size-3 opacity-60"
			role="img"
			aria-label="Switch"
		>
			<path d="m7 15 5 5 5-5" />
			<path d="m7 9 5-5 5 5" />
		</svg>
	);
}
