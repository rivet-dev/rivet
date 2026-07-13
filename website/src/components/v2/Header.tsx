"use client";
import { usePathname } from "@/hooks/usePathname";
import { ActiveLink } from "@/components/ActiveLink";
import { Tree } from "@/components/DocsNavigation";
import { NavigationStateProvider } from "@/providers/NavigationStateProvider";
import type { SidebarItem } from "@/lib/sitemap";
import logoUrl from "@/images/rivet-logos/icon-text-white.svg";
import logoTextBlackUrl from "@/images/rivet-logos/icon-text-black.svg";
import logoIconUrl from "@/images/rivet-logos/icon-white.svg";
import { cn } from "@rivet-gg/components";
import { Header as RivetHeader } from "@rivet-gg/components/header";
import { Icon, faDiscord } from "@rivet-gg/icons";
import React, { type ReactNode, useEffect, useRef, useState } from "react";
import {
	Button,
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuTrigger,
} from "@rivet-gg/components";
import { faChevronDown } from "@rivet-gg/icons";
import { ArrowRight } from "lucide-react";
import actorsLogoUrl from "@/images/products/actors-logo.svg";
import agentosLogoUrl from "@/images/products/agentos-logo.svg";
import cloudLogoUrl from "@/images/products/rivet-cloud-logo.svg";
import sandboxAgentLogoUrl from "@/images/products/sandbox-agent-logo.svg";
import { GitHubDropdown } from "./GitHubDropdown";
import { HeaderSearch } from "./HeaderSearch";
import { LogoContextMenu } from "./LogoContextMenu";
import { DocsTabs } from "@/components/DocsTabs";

interface TextNavItemProps {
	href: string;
	children: ReactNode;
	className?: string;
	ariaCurrent?: boolean | "page" | "step" | "location" | "date" | "time";
}

function TextNavItem({
	href,
	children,
	className,
	ariaCurrent,
}: TextNavItemProps) {
	return (
		<div className={cn("px-2.5 py-2", className)}>
			<RivetHeader.NavItem asChild>
				<a
					href={href}
					className={cn(
						"text-zinc-400 hover:text-white transition-colors duration-200",
						ariaCurrent === "page" && "text-white",
					)}
					aria-current={ariaCurrent}
				>
					{children}
				</a>
			</RivetHeader.NavItem>
		</div>
	);
}

function ProductsDropdown({
	active,
	lightTheme = false,
	align = "center",
}: {
	active?: boolean;
	lightTheme?: boolean;
	align?: "center" | "start";
}) {
	const [isOpen, setIsOpen] = useState(false);
	const closeTimeoutRef = useRef<NodeJS.Timeout | null>(null);
	const isHoveringRef = useRef(false);

	const products = [
		{
			label: "Actors",
			href: "/actors",
			logo: actorsLogoUrl,
			description: "Where agents live: state, identity, durability",
		},
		{
			label: "agentOS",
			href: "https://agentos-sdk.dev",
			logo: agentosLogoUrl,
			description: "How agents act: file system, shell, tools",
			external: true,
		},
		{
			label: "Rivet Cloud",
			href: "/cloud",
			logo: cloudLogoUrl,
			description: "Where they run: managed compute",
		},
	];

	const cancelClose = () => {
		if (closeTimeoutRef.current) {
			clearTimeout(closeTimeoutRef.current);
			closeTimeoutRef.current = null;
		}
	};

	const scheduleClose = () => {
		cancelClose();
		closeTimeoutRef.current = setTimeout(() => {
			setIsOpen(false);
		}, 150);
	};

	const handleMouseEnter = () => {
		isHoveringRef.current = true;
		cancelClose();
		setIsOpen(true);
	};

	const handleMouseLeave = () => {
		isHoveringRef.current = false;
		scheduleClose();
	};

	const handleOpenChange = (open: boolean) => {
		if (!open) {
			cancelClose();
			setIsOpen(false);
		}
	};

	const handlePointerDown = (e: React.PointerEvent) => {
		e.preventDefault();
		cancelClose();
		setIsOpen((prev) => !prev);
	};

	useEffect(() => {
		return () => cancelClose();
	}, []);

	if (lightTheme) {
		return (
			<div
				className={cn("group/products px-2.5 py-2", align === "start" && "relative")}
				onMouseEnter={handleMouseEnter}
				onMouseLeave={handleMouseLeave}
			>
				<RivetHeader.NavItem asChild>
					<button
						type="button"
						aria-expanded={isOpen}
						className={cn(
							"cursor-default flex items-center gap-1 relative transition-colors duration-200",
							"!text-zinc-600 hover:!text-zinc-900",
							active && "!text-zinc-900",
							// Invisible hover bridge spanning the visual gap down to the
							// dropdown panel so moving the mouse from the trigger to the
							// panel does not cross a dead zone and close the menu.
							"after:absolute after:left-0 after:right-0 after:top-full after:h-7 after:content-['']",
						)}
						onMouseEnter={handleMouseEnter}
					>
						Products
						<Icon
							aria-hidden="true"
							icon={faChevronDown}
							className={cn(
								"h-3 w-3 ml-0.5 transition-transform duration-200",
								isOpen && "rotate-180",
							)}
						/>
					</button>
				</RivetHeader.NavItem>
				<div
					className={cn(
						"z-50 -translate-y-1 overflow-hidden rounded-2xl border border-ink/10 bg-paper/80 p-1.5 opacity-0 shadow-[0_18px_50px_-32px_rgba(27,25,22,0.42)] backdrop-blur-[18px] backdrop-saturate-[1.35] transition-all duration-150 pointer-events-none group-hover/products:pointer-events-auto group-hover/products:translate-y-0 group-hover/products:opacity-100",
						align === "start"
							? "absolute left-0 top-full mt-3 w-80"
							: "fixed left-1/2 top-[63px] w-[min(912px,calc(100vw-3rem))] -translate-x-1/2",
						isOpen
							? "pointer-events-auto translate-y-0 opacity-100"
							: "pointer-events-none -translate-y-1 opacity-0",
					)}
					onMouseEnter={handleMouseEnter}
					onMouseLeave={handleMouseLeave}
				>
					<div className="flex flex-col">
						{products.map((product) => (
							<a
								key={product.href}
								href={product.href}
								className="group/product-row flex items-center gap-2.5 rounded-xl px-3 py-1 text-ink transition-colors hover:bg-ink/[0.07]"
							>
								<img
									src={product.logo.src}
									alt={product.label}
									width={18}
									height={18}
									className="h-[18px] w-[18px] shrink-0 invert opacity-85"
									loading="lazy"
									decoding="async"
								/>
								<div className="min-w-0 flex-1">
									<div className="text-sm font-medium leading-tight text-ink">
										{product.label}
									</div>
									<div className="text-xs leading-tight text-ink-faint">
										{product.description}
									</div>
								</div>
								<ArrowRight
									aria-hidden="true"
									className="invisible h-4 w-4 text-ink-faint opacity-0 transition-all duration-150 group-hover/product-row:visible group-hover/product-row:translate-x-0.5 group-hover/product-row:opacity-100 group-hover/product-row:text-ink"
								/>
							</a>
						))}
					</div>
				</div>
			</div>
		);
	}

	return (
		<div
			className="px-2.5 py-2"
			onMouseEnter={handleMouseEnter}
			onMouseLeave={handleMouseLeave}
		>
			<DropdownMenu open={isOpen} onOpenChange={handleOpenChange} modal={false}>
				<DropdownMenuTrigger asChild>
					<RivetHeader.NavItem asChild>
						<button
							type="button"
							className={cn(
								"cursor-pointer flex items-center gap-1 relative transition-colors duration-200",
								lightTheme ? "!text-zinc-600 hover:!text-zinc-900" : "!text-zinc-400 hover:!text-white",
								active && !lightTheme && "!text-white",
								"after:absolute after:left-0 after:right-0 after:top-full after:h-4 after:content-['']",
							)}
							onPointerDown={handlePointerDown}
							onMouseEnter={handleMouseEnter}
						>
							Products
							<Icon icon={faChevronDown} className="h-3 w-3 ml-0.5" />
						</button>
					</RivetHeader.NavItem>
				</DropdownMenuTrigger>
				<DropdownMenuContent
					align="start"
					className={cn(
						"min-w-[280px] p-4 rounded-xl shadow-xl",
						lightTheme
							? "bg-white/95 backdrop-blur-lg border border-zinc-200"
							: "bg-black/95 backdrop-blur-lg border border-white/10",
					)}
					onMouseEnter={handleMouseEnter}
					onMouseLeave={handleMouseLeave}
					sideOffset={0}
					alignOffset={0}
					side="bottom"
				>
					<div className="flex flex-col gap-1">
						{products.map((product) => (
							<React.Fragment key={product.href}>
								<a
									href={product.href}
									className={cn(
										"group flex items-center gap-3 p-3 rounded-lg transition-colors cursor-pointer",
										lightTheme ? "hover:bg-zinc-100" : "hover:bg-white/5",
									)}
								>
									<img
										src={product.logo.src}
										alt={product.label}
										width={24}
										height={24}
										className="h-6 w-6"
										loading="lazy"
										decoding="async"
									/>
									<div className="flex flex-col">
										<div className={cn(
											"font-medium text-sm transition-colors",
											lightTheme ? "text-zinc-900" : "text-white group-hover:text-white",
										)}>
											{product.label}
										</div>
										<div className={cn(
											"text-xs transition-colors leading-relaxed",
											lightTheme ? "text-zinc-500 group-hover:text-zinc-700" : "text-zinc-400 group-hover:text-zinc-300",
										)}>
											{product.description}
										</div>
									</div>
								</a>
								{product.subItems?.map((sub) => (
									<a
										key={sub.href}
										href={sub.href}
										className={cn(
											"group flex items-center gap-2.5 py-1.5 pl-12 pr-3 rounded-lg transition-colors cursor-pointer",
											lightTheme ? "hover:bg-zinc-100" : "hover:bg-white/5",
										)}
									>
										<sub.icon
											className={cn(
												"h-3.5 w-3.5 transition-colors",
												lightTheme ? "text-zinc-500 group-hover:text-zinc-700" : "text-zinc-500 group-hover:text-zinc-300",
											)}
										/>
										<span
											className={cn(
												"text-xs transition-colors",
												lightTheme ? "text-zinc-500 group-hover:text-zinc-700" : "text-zinc-400 group-hover:text-zinc-300",
											)}
										>
											{sub.label}
										</span>
									</a>
								))}
							</React.Fragment>
						))}
					</div>
				</DropdownMenuContent>
			</DropdownMenu>
		</div>
	);
}

interface HeaderProps {
	active?:
	| "product"
	| "docs"
	| "cookbook"
	| "blog"
	| "pricing"
	| "learn";
	subnav?: ReactNode;
	mobileSidebar?: ReactNode;
	sidebarData?: SidebarItem[];
	variant?: "floating" | "full-width";
	learnMode?: boolean;
	showDocsTabs?: boolean;
	light?: boolean;
	initialPathname?: string;
}

export function Header({
	active,
	subnav,
	mobileSidebar,
	sidebarData,
	variant = "full-width",
	learnMode = false,
	showDocsTabs = false,
	light = false,
	initialPathname = "",
}: HeaderProps) {
	const [isScrolled, setIsScrolled] = useState(false);

	useEffect(() => {
		if (variant === "floating") {
			const handleScroll = () => {
				setIsScrolled(window.scrollY > 20);
			};

			window.addEventListener("scroll", handleScroll);
			return () => window.removeEventListener("scroll", handleScroll);
		}
	}, [variant]);

	const clientPathname = usePathname();
	const pathname = clientPathname || initialPathname;
	// The floating variant only renders on marketing pages, which are all
	// porcelain. The full-width variant is porcelain for docs (light=true) and
	// stays dark for the Learn section.
	const isLightTheme = variant === "floating" || light;

	// Use DocsTabs as subnav if showDocsTabs is true
	const effectiveSubnav = showDocsTabs ? <DocsTabs light={isLightTheme} initialPathname={pathname} /> : subnav;

	// Set body attribute for global CSS targeting (e.g., mobile sheet styling)
	useEffect(() => {
		if (isLightTheme) {
			document.body.setAttribute('data-light-theme', 'true');
		} else {
			document.body.removeAttribute('data-light-theme');
		}
		return () => {
			document.body.removeAttribute('data-light-theme');
		};
	}, [isLightTheme]);

	if (variant === "floating") {
		const headerStyles = cn(
			"border-transparent static bg-transparent rounded-2xl max-w-[960px] md:max-w-[1200px] [&>div:first-child]:px-3 backdrop-blur-none transition-all hover:opacity-100",
			isScrolled ? "opacity-100" : "opacity-100 md:opacity-80",
		);

		return (
			<div
				className={cn(
					"fixed top-2 z-50 w-full max-w-[960px] px-3 md:left-1/2 md:top-4 md:-translate-x-1/2 md:px-6",
					isLightTheme && "selection:bg-orange-200 selection:text-orange-900"
				)}
				data-light-theme={isLightTheme ? "true" : undefined}
			>
				<div
					className={cn(
						"hero-bg-exclude",
						'relative before:pointer-events-none before:absolute before:inset-[-1px] before:z-20 before:block before:rounded-2xl before:border before:border-ink/10 before:content-[""] before:transition-colors before:duration-300 before:ease-in-out',
					)}
				>
					{/* White glass pill: frosted fill with a soft top sheen. The pill's
						outline is the ink/10 hairline on the parent's ::before, so this
						layer carries no border of its own. */}
					<div className="absolute inset-0 -z-[1] overflow-hidden rounded-2xl bg-white/60 shadow-[inset_0_1px_0_rgba(255,255,255,0.55)] backdrop-blur-[18px] backdrop-saturate-[1.4]" />
					<RivetHeader
						className={headerStyles}
						logo={
							<>
								{/* Mobile logo */}
								<div className="md:hidden ml-1">
									<a href="/">
										<img
											src={logoTextBlackUrl.src}
											width={80}
											height={24}
											className="w-20 shrink-0"
											alt="Rivet logo"
										/>
									</a>
								</div>
								{/* Desktop logo */}
								<div className="hidden md:block">
									<LogoContextMenu>
										<a href="/">
											<img
												src={logoTextBlackUrl.src}
												width={80}
												height={24}
												className="ml-1 w-20 shrink-0"
												alt="Rivet logo"
											/>
										</a>
									</LogoContextMenu>
								</div>
							</>
						}
						subnav={effectiveSubnav}
						support={null}
						links={
							<div className="flex flex-row items-center">
								{variant === "full-width" && <HeaderSearch />}
								<RivetHeader.NavItem asChild className="p-2 mr-4 hidden md:flex">
									<a href="https://rivet.dev/discord" className="!text-ink-soft hover:!text-ink transition-colors">
										<Icon icon={faDiscord} />
									</a>
								</RivetHeader.NavItem>
								<GitHubDropdown className="hidden md:inline-flex items-center justify-center whitespace-nowrap rounded-md border px-4 py-2 h-10 text-sm mr-2 transition-colors border-ink/15 text-ink-soft hover:border-ink/30 hover:text-ink" />
								<a
									href="https://dashboard.rivet.dev"
									className="font-v2 subpixel-antialiased inline-flex items-center justify-center whitespace-nowrap rounded-md bg-ink px-4 py-2 text-sm text-cream hover:bg-ink/85 transition-colors"
								>
									Sign In
								</a>
							</div>
						}
						mobileBreadcrumbs={
							<DocsMobileNavigation
								tree={mobileSidebar}
								sidebarData={sidebarData}
								isLightTheme={isLightTheme}
							/>
						}
						sheetClassName="!bg-paper [&>button]:!bg-paper [&>button]:!text-ink [&>button]:!border-ink/15"
						lightTheme={isLightTheme}
						breadcrumbs={
							<div className="flex items-center font-v2 subpixel-antialiased [&_a]:!text-ink-soft [&_a:hover]:!text-ink [&_a[aria-current=page]]:!text-ink [&_button]:!text-ink-soft">
								<ProductsDropdown active={active === "product"} lightTheme />
								<TextNavItem
									href="/docs"
									ariaCurrent={active === "docs" ? "page" : undefined}
								>
									Documentation
								</TextNavItem>
								<TextNavItem href="/enterprise">
									Enterprise
								</TextNavItem>
								<TextNavItem
									href="/cloud"
									ariaCurrent={active === "pricing" ? "page" : undefined}
								>
									Pricing
								</TextNavItem>
							</div>
						}
					/>
				</div>
			</div>
		);
	}

	// Full-width variant
	return (
		<RivetHeader
			className={cn(
				"sticky top-0 z-50 backdrop-blur-lg",
				isLightTheme
					? "bg-paper/95 border-b border-ink/10 [&_button[data-mobile-menu-trigger]]:text-ink"
					: "bg-neutral-950/80",
				"[&>div:first-child]:px-3 md:[&>div:first-child]:max-w-none md:[&>div:first-child]:px-0 md:px-8",
				// 0 padding on bottom for larger screens when subnav is showing
				effectiveSubnav ? "pb-2 md:pb-0 md:pt-3 md:[&>div:first-child>div:first-child]:min-h-12 md:[&>div:first-child>div:first-child]:mb-3" : "md:py-4",
				// Learn mode styling
				!isLightTheme && learnMode && "bg-[#1c1917] border-b border-[#44403c]",
			)}
			logo={
				<div className="hidden md:block">
					<LogoContextMenu>
						<a href="/">
							<img
								src={isLightTheme ? logoTextBlackUrl.src : logoUrl.src}
								width={80}
								height={24}
								className="ml-1 w-20 shrink-0"
								alt="Rivet logo"
								loading="eager"
								decoding="async"
							/>
						</a>
					</LogoContextMenu>
				</div>
			}
			subnav={effectiveSubnav}
			support={<></>}
			links={
				<div className="flex flex-row items-center">
					{!learnMode && (
						<div className="mr-4">
							<HeaderSearch light={isLightTheme} />
						</div>
					)}
					<RivetHeader.NavItem asChild className="p-2 mr-4">
						<a
							href="https://rivet.dev/discord"
							className={isLightTheme ? "!text-ink-soft hover:!text-ink transition-colors" : "text-white/90"}
						>
							<Icon icon={faDiscord} className="drop-shadow-md" />
						</a>
					</RivetHeader.NavItem>
					<GitHubDropdown
						className={cn(
							"inline-flex items-center justify-center whitespace-nowrap rounded-md border px-4 py-2 h-10 text-sm mr-2 transition-colors",
							isLightTheme
								? "border-ink/15 text-ink-soft hover:border-ink/30 hover:text-ink"
								: "border-white/10 hover:border-white/20 text-white/90 hover:text-white",
						)}
					/>
					<a
						href="https://dashboard.rivet.dev"
						className={cn(
							"font-v2 subpixel-antialiased inline-flex items-center justify-center whitespace-nowrap rounded-md px-4 py-2 text-sm transition-colors",
							isLightTheme
								? "bg-ink text-cream hover:bg-ink/85"
								: "border border-white/10 bg-white/5 text-white shadow-sm hover:border-white/20",
						)}
					>
						Sign In
					</a>
				</div>
			}
			lightTheme={isLightTheme}
			sheetClassName={isLightTheme ? "!bg-paper [&>button]:!bg-paper [&>button]:!text-ink [&>button]:!border-ink/15" : undefined}
			mobileBreadcrumbs={<DocsMobileNavigation tree={mobileSidebar} sidebarData={sidebarData} isLightTheme={isLightTheme} />}
			breadcrumbs={
				<div className={cn(
					"flex items-center font-v2 subpixel-antialiased",
					isLightTheme && "[&_a]:!text-ink-soft [&_a:hover]:!text-ink [&_a[aria-current=page]]:!text-ink [&_button]:!text-ink-soft",
				)}>
					<ProductsDropdown active={active === "product"} lightTheme={isLightTheme} align="start" />
					<TextNavItem
						href="/docs"
						ariaCurrent={active === "docs" ? "page" : undefined}
					>
						Documentation
					</TextNavItem>
					<TextNavItem href="/enterprise">
						Enterprise
					</TextNavItem>
					<TextNavItem
						href="/cloud"
						ariaCurrent={active === "pricing" ? "page" : undefined}
					>
						Pricing
					</TextNavItem>
				</div>
			}
		/>
	);
}

function DocsMobileNavigation({
	tree,
	sidebarData,
	isLightTheme = false,
}: {
	tree?: ReactNode;
	sidebarData?: SidebarItem[];
	isLightTheme?: boolean;
}) {
	const pathname = usePathname() || "";
	const isDocsPage = pathname.startsWith("/docs");

	// Determine current section based on pathname
	const getCurrentSection = () => {
		if (pathname.startsWith("/docs/actors")) return "actors";
		if (pathname.startsWith("/docs/integrations")) return "integrations";
		if (pathname.startsWith("/docs/api")) return "api";
		if (pathname.startsWith("/docs/quickstart")) return "quickstart";
		return "overview";
	};

	const sections = [
		{ id: "overview", label: "Overview", href: "/docs" },
		{ id: "quickstart", label: "Quickstart", href: "/docs/quickstart" },
		{ id: "actors", label: "Actors", href: "/docs/actors" },
		{ id: "integrations", label: "Integrations", href: "/docs/integrations" },
		{ id: "api", label: "API Reference", href: "/docs/api" },
	];

	const mainLinks = [
		{ href: "/docs", label: "Documentation" },
		{ href: "/enterprise", label: "Enterprise" },
		{ href: "/cloud", label: "Pricing" },
	];

	const products = [
		{ label: "Actors", href: "/actors", logo: actorsLogoUrl },
		{
			label: "agentOS",
			href: "https://agentos-sdk.dev",
			logo: agentosLogoUrl,
			external: true,
		},
		{ label: "Rivet Cloud", href: "/cloud", logo: cloudLogoUrl },
		{
			label: "Sandbox Agent SDK",
			href: "https://sandboxagent.dev/",
			logo: sandboxAgentLogoUrl,
			external: true,
		},
		{
			label: "Secure Exec SDK",
			href: "https://secureexec.dev/",
			logo: sandboxAgentLogoUrl,
			external: true,
		},
	];

	const currentSection = sections.find((s) => s.id === getCurrentSection());

	if (isLightTheme) {
		return (
			<div className="flex flex-col gap-2 font-v2 subpixel-antialiased text-sm">
				{/* Home logo */}
				<a href="/" className="py-3 px-2">
					<img
						src={logoTextBlackUrl.src}
						alt="Rivet"
						width={80}
						height={24}
						className="w-20"
					/>
				</a>

				{/* Products section */}
				<div className="text-ink-faint py-2 px-2 text-xs uppercase tracking-wide">
					Products
				</div>
				{products.map((product) => (
					<a
						key={product.href}
						href={product.href}
						target={product.external ? "_blank" : undefined}
						rel={product.external ? "noopener noreferrer" : undefined}
						className="text-ink py-2 px-2 pl-4 hover:bg-ink/5 rounded-sm transition-colors flex items-center gap-2"
					>
						<img
							src={product.logo.src}
							alt={product.label}
							width={16}
							height={16}
							className="h-4 w-4 invert opacity-85"
							loading="lazy"
							decoding="async"
						/>
						{product.label}
					</a>
				))}

				{/* Main navigation links */}
				{mainLinks.map(({ href, label }) => (
					<a
						key={href}
						href={href}
						className="text-ink py-2 px-2 hover:bg-ink/5 rounded-sm transition-colors"
					>
						{label}
					</a>
				))}

				{/* Docs section dropdown + sidebar tree */}
				{isDocsPage && (
					<>
						<div className="border-t-2 border-ink/10 my-2" />

						{/* Section dropdown */}
						<DropdownMenu>
							<DropdownMenuTrigger asChild>
								<Button
									variant="outline"
									className="w-full justify-between h-9 text-sm border-ink/15 bg-white/55 text-ink hover:bg-white/70 hover:border-ink/30"
								>
									{currentSection?.label || "Select Section"}
									<Icon icon={faChevronDown} className="h-3.5 w-3.5 ml-2" />
								</Button>
							</DropdownMenuTrigger>
							<DropdownMenuContent className="w-[calc(100vw-3rem)] bg-white border border-ink/10 text-ink [&_[role=menuitem]]:text-ink [&_[role=menuitem][data-highlighted]]:bg-ink/[0.06] [&_[role=menuitem][data-highlighted]]:text-ink">
								{sections.map(({ id, label, href }) => (
									<DropdownMenuItem key={id} asChild>
										<a href={href}>{label}</a>
									</DropdownMenuItem>
								))}
							</DropdownMenuContent>
						</DropdownMenu>

						{/* Tree/sidebar content */}
						{tree && <div className="mt-1">{tree}</div>}
						{!tree && sidebarData && (
							<NavigationStateProvider>
								<div className="mt-1">
									<Tree pages={sidebarData} />
								</div>
							</NavigationStateProvider>
						)}
					</>
				)}

				{/* Dashboard button */}
				<div className="mt-4 pt-4 border-t border-ink/10">
					<a
						href="https://dashboard.rivet.dev/"
						className="flex items-center justify-center w-full rounded-md bg-ink px-4 py-2 text-sm font-medium text-cream transition-colors hover:bg-ink/85"
					>
						Dashboard
					</a>
				</div>
			</div>
		);
	}

	return (
		<div className="flex flex-col gap-2 font-v2 subpixel-antialiased text-sm">
			{/* Home logo - full logo on small screens, icon only on tablet */}
			<a href="/" className="py-3 px-2">
				<img
					src={logoUrl.src}
					alt="Rivet"
					width={80}
					height={24}
					className="w-20 sm:hidden"
				/>
				<img
					src={logoIconUrl.src}
					alt="Rivet"
					width={32}
					height={32}
					className="w-8 h-8 hidden sm:block"
				/>
			</a>

			{/* Products section */}
			<div className="text-zinc-500 py-2 px-2 text-xs uppercase tracking-wide">
				Products
			</div>
			{products.map((product) => (
				<a
					key={product.href}
					href={product.href}
					target={product.external ? "_blank" : undefined}
					rel={product.external ? "noopener noreferrer" : undefined}
					className="text-white py-2 px-2 pl-4 hover:bg-white/5 rounded-sm transition-colors flex items-center gap-2"
				>
					<img
						src={product.logo.src}
						alt={product.label}
						width={16}
						height={16}
						className="h-4 w-4"
						loading="lazy"
						decoding="async"
					/>
					{product.label}
				</a>
			))}

			{/* Main navigation links */}
			{mainLinks.map(({ href, label }) => (
				<a
					key={href}
					href={href}
					className="text-white py-2 px-2 hover:bg-white/5 rounded-sm transition-colors"
				>
					{label}
				</a>
			))}

			{/* Separator and docs content */}
			{isDocsPage && (
				<>
					<div className="border-t-2 border-white/10 my-2" />

					{/* Section dropdown */}
					<DropdownMenu>
						<DropdownMenuTrigger asChild>
							<Button
								variant="outline"
								className="w-full justify-between h-9 text-sm border-white/10 bg-white/5 text-white hover:bg-white/10 hover:border-white/20"
							>
								{currentSection?.label || "Select Section"}
								<Icon icon={faChevronDown} className="h-3.5 w-3.5 ml-2" />
							</Button>
						</DropdownMenuTrigger>
						<DropdownMenuContent className="w-[calc(100vw-3rem)] bg-black/95 backdrop-blur-lg border-white/10">
							{sections.map(({ id, label, href }) => (
								<DropdownMenuItem
									key={id}
									asChild
									className="text-white hover:bg-white/5 focus:bg-white/5"
								>
									<a href={href}>{label}</a>
								</DropdownMenuItem>
							))}
						</DropdownMenuContent>
					</DropdownMenu>

					{/* Tree/sidebar content */}
					{tree && <div className="mt-1">{tree}</div>}
					{!tree && sidebarData && (
						<NavigationStateProvider>
							<div className="mt-1">
								<Tree pages={sidebarData} />
							</div>
						</NavigationStateProvider>
					)}
				</>
			)}

			{/* Dashboard button */}
			<div className="mt-4 pt-4 border-t border-white/10">
				<a
					href="https://dashboard.rivet.dev/"
					className="flex items-center justify-center w-full rounded-md bg-white px-4 py-2 text-sm font-medium text-black transition-colors hover:bg-zinc-200"
				>
					Dashboard
				</a>
			</div>
		</div>
	);
}
