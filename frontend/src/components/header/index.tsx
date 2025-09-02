import { faBars, Icon } from "@rivet-gg/icons";
import type { ReactNode } from "react";
import { AssetImage } from "../asset-image";
import { cn } from "../lib/utils";
import { Button } from "../ui/button";
import { Flex } from "../ui/flex";
import { Sheet, SheetContent, SheetTrigger } from "../ui/sheet";
import { HeaderLink } from "./header-link";
import { HeaderProgress } from "./header-progress";
import { NavItem } from "./nav-item";

export { NavItem };

interface HeaderProps {
	className?: string;
	mobileBreadcrumbs?: ReactNode;
	breadcrumbs?: ReactNode;
	subnav?: ReactNode;
	addons?: ReactNode;
	links?: ReactNode;
	suffix?: ReactNode;
	logo?: ReactNode;
	support?: ReactNode;
}

export function Header({
	className,
	breadcrumbs,
	subnav,
	mobileBreadcrumbs,
	addons,
	links,
	suffix,
	logo,
	support = (
		<Flex direction="col" justify="end" gap="6">
			<NavItem asChild>
				<a
					href="https://rivet.gg/docs"
					target="_blank"
					rel="noreferrer"
				>
					Docs
				</a>
			</NavItem>
			<NavItem asChild>
				<a
					href="https://rivet.gg/support"
					target="_blank"
					rel="noreferrer"
				>
					Support
				</a>
			</NavItem>
		</Flex>
	),
}: HeaderProps) {
	return (
		<header
			className={cn(
				"bg-background/60 border-b-border sticky top-0 z-10 flex flex-col items-center border-b backdrop-blur",
				"pt-2",
				"pb-2",
				className,
			)}
		>
			{addons}
			<div className="w-full px-8 flex min-h-10 flex-col justify-center">
				<div className="flex w-full items-center gap-4">
					<Sheet>
						<SheetTrigger asChild>
							<Button
								variant="outline"
								size="icon"
								className="shrink-0 md:hidden text-foreground"
							>
								<Icon icon={faBars} className="size-5" />
								<span className="sr-only">
									Toggle navigation menu
								</span>
							</Button>
						</SheetTrigger>
						<SheetContent side="left" className="overflow-auto p-0">
							<nav className="min-h-full text-lg font-medium h-full max-w-full">
								<div className="flex flex-col min-h-full">
									<a
										href="/"
										className="flex sticky p-6 top-0 z-10 bg-background/10 backdrop-blur block w-full items-center gap-2 text-lg font-semibold"
									>
										{logo}
									</a>
									<div className="flex flex-1 flex-col px-6 gap-6">
										{mobileBreadcrumbs}
									</div>
									<div className="px-6 py-6">{support}</div>
								</div>
							</nav>
						</SheetContent>
					</Sheet>
					<nav className="flex-1 font-medium md:flex md:flex-row md:items-center md:gap-3 md:text-sm lg:gap-4">
						{logo ? (
							logo
						) : (
							<a href="/">
								<AssetImage
									className="h-6"
									alt="Rivet Logo"
									src="/logo/cream.svg"
								/>
							</a>
						)}
						{breadcrumbs ? (
							<div className="hidden md:flex">{breadcrumbs}</div>
						) : null}
					</nav>
					<div className="gap-6 font-medium md:flex md:flex-row md:items-center md:gap-5 md:text-sm">
						{links}
					</div>
				</div>
				{subnav}
			</div>
			{suffix}
		</header>
	);
}

Header.NavItem = NavItem;
Header.Progress = HeaderProgress;
Header.Link = HeaderLink;
