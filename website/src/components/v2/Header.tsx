"use client";
import { usePathname } from "@/hooks/usePathname";
import { ActiveLink } from "@/components/ActiveLink";
import { Tree } from "@/components/DocsNavigation";
import { NavigationStateProvider } from "@/providers/NavigationStateProvider";
import type { SidebarItem } from "@/lib/sitemap";
import { registry } from "@/data/registry";
import logoUrl from "@/images/rivet-logos/icon-text-white.svg";
import logoIconUrl from "@/images/rivet-logos/icon-white.svg";
import logoIconWhiteUrl from "@/images/rivet-logos/icon-white.svg";

const WHITE_THEMED_PATHS = ['/agent-os', '/agent-os/use-cases', '/agent-os/pricing', '/agent-os/registry', '/from-unix-to-agents', '/install'];
const REGISTRY_PACKAGE_COUNT = registry.length;
const AGENT_OS_DOCS_HREF = "/docs/agent-os";
const AGENT_OS_REGISTRY_HREF = "/agent-os/registry";
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
import actorsLogoUrl from "@/images/products/actors-logo.svg";
import agentosLogoUrl from "@/images/products/agentos-logo.svg";
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
		<div
			className={cn(
				"px-2.5 py-2 opacity-60 hover:opacity-100 transition-all duration-200",
				className,
			)}
		>
			<RivetHeader.NavItem asChild>
				<a href={href} className="text-white" aria-current={ariaCurrent}>
					{children}
				</a>
			</RivetHeader.NavItem>
		</div>
	);
}

function ProductsDropdown({
	active,
	lightTheme = false,
}: {
	active?: boolean;
	lightTheme?: boolean;
}) {
	const [isOpen, setIsOpen] = useState(false);
	const closeTimeoutRef = useRef<NodeJS.Timeout | null>(null);
	const isHoveringRef = useRef(false);

	const products = [
		{
			label: "Actors",
			href: "/actors",
			logo: actorsLogoUrl,
			description: "Build stateful backends",
		},
		{
			label: "agentOS",
			href: "/agent-os",
			logo: agentosLogoUrl,
			description: "Everything agents need to run and operate",
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

	return (
		<div
			className={cn(
				"px-2.5 py-2 opacity-60 hover:opacity-100 transition-all duration-200",
				lightTheme && "opacity-100",
			)}
			onMouseEnter={handleMouseEnter}
			onMouseLeave={handleMouseLeave}
		>
			<DropdownMenu open={isOpen} onOpenChange={handleOpenChange} modal={false}>
				<DropdownMenuTrigger asChild>
					<RivetHeader.NavItem asChild>
						<button
							type="button"
							className={cn(
								"cursor-pointer flex items-center gap-1 relative",
								lightTheme ? "!text-zinc-600 hover:!text-zinc-900" : "!text-white",
								active && "opacity-100",
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
	initialPathname = "",
}: HeaderProps) {
	const [isScrolled, setIsScrolled] = useState(false);

	// Use DocsTabs as subnav if showDocsTabs is true
	const effectiveSubnav = showDocsTabs ? <DocsTabs /> : subnav;

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
	const isLightTheme = WHITE_THEMED_PATHS.some((p) => pathname === p || pathname === p + '/') || pathname.startsWith('/agent-os/registry/');
	const isRegistryPage =
		pathname === AGENT_OS_REGISTRY_HREF || pathname === `${AGENT_OS_REGISTRY_HREF}/`;

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
			"md:border-transparent md:static md:bg-transparent md:rounded-2xl md:max-w-[1200px] md:border-transparent md:backdrop-none [&>div:first-child]:px-3 md:backdrop-blur-none transition-all hover:opacity-100",
			isScrolled ? "opacity-100" : "opacity-80",
		);

		return (
			<div
				className={cn(
					"fixed top-0 z-50 w-full max-w-[1200px] md:left-1/2 md:top-4 md:-translate-x-1/2 md:px-8",
					isLightTheme && "selection:bg-orange-200 selection:text-orange-900"
				)}
				data-light-theme={isLightTheme ? "true" : undefined}
			>
				<div
					className={cn(
						"hero-bg-exclude",
						isLightTheme
							? 'relative before:pointer-events-none before:absolute before:inset-[-1px] before:z-20 before:hidden before:rounded-2xl before:border before:border-zinc-200 before:content-[""] before:transition-colors before:duration-300 before:ease-in-out md:before:block'
							: 'relative before:pointer-events-none before:absolute before:inset-[-1px] before:z-20 before:hidden before:rounded-2xl before:border before:border-white/10 before:content-[""] before:transition-colors before:duration-300 before:ease-in-out md:before:block',
					)}
				>
					<div className={cn("absolute inset-0 -z-[1] hidden overflow-hidden rounded-2xl backdrop-blur-lg md:block", isLightTheme ? "bg-white/80" : "bg-background/80")} />
					<RivetHeader
						className={cn(
							headerStyles,
							isLightTheme && "md:bg-transparent [&_button[data-mobile-menu-trigger]]:text-zinc-900 bg-white/95 backdrop-blur-lg md:backdrop-blur-none"
						)}
						logo={
							<>
								{/* Mobile logo */}
								<div className="md:hidden ml-1">
									{isLightTheme ? (
										<a href="/" className="flex items-center gap-2">
											<img
												src={logoIconWhiteUrl.src}
												width={24}
												height={24}
												className="h-6 w-6"
												alt="Rivet logo"
											/>
											<div className="h-4 w-px bg-zinc-300" />
											<img
												src="/images/agent-os/agentos-hero-logo.svg"
												className="h-4 w-auto"
												alt="agentOS"
											/>
										</a>
									) : (
										<a href="/">
											<img
												src={logoUrl.src}
												width={80}
												height={24}
												className="w-20 shrink-0"
												alt="Rivet logo"
											/>
										</a>
									)}
								</div>
								{/* Desktop logo */}
								<div className="hidden md:block">
									{isLightTheme ? (
										<div className="ml-1 flex items-center gap-3">
											<a href="/">
												<img
													src={logoIconWhiteUrl.src}
													width={24}
													height={24}
													className="h-6 w-6"
													alt="Rivet logo"
												/>
											</a>
											<div className="h-5 w-px bg-zinc-300" />
											<a href="/agent-os">
												<img
													src="/images/agent-os/agentos-hero-logo.svg"
													className="h-5 w-auto"
													alt="agentOS"
												/>
											</a>
										</div>
									) : (
										<LogoContextMenu>
											<a href="/">
												<img
													src={logoUrl.src}
													width={80}
													height={24}
													className="ml-1 w-20 shrink-0"
													alt="Rivet logo"
												/>
											</a>
										</LogoContextMenu>
									)}
								</div>
							</>
						}
						subnav={effectiveSubnav}
						support={null}
						links={
							<div className="flex flex-row items-center">
								{variant === "full-width" && <HeaderSearch />}
								<RivetHeader.NavItem asChild className="p-2 mr-4">
									<a href="https://rivet.dev/discord" className={isLightTheme ? "text-zinc-500 hover:text-zinc-900" : "text-white/90"}>
										<Icon icon={faDiscord} className="drop-shadow-md" />
									</a>
								</RivetHeader.NavItem>
								<GitHubDropdown className={cn("inline-flex items-center justify-center whitespace-nowrap rounded-md border px-4 py-2 h-10 text-sm mr-2 transition-colors", isLightTheme ? "border-zinc-200 text-zinc-500 hover:border-zinc-300 hover:text-zinc-900" : "border-white/10 text-white/90 hover:border-white/20 hover:text-white")} />
								{isLightTheme ? (
									<a
										href="/install"
										className="font-v2 subpixel-antialiased inline-flex items-center justify-center whitespace-nowrap rounded-md border border-zinc-200 bg-zinc-900 px-4 py-2 text-sm text-white shadow-sm hover:bg-zinc-700 transition-colors"
									>
										Install
									</a>
								) : (
									<a
										href="https://dashboard.rivet.dev"
										className="font-v2 subpixel-antialiased inline-flex items-center justify-center whitespace-nowrap rounded-md border border-white/10 bg-white/5 px-4 py-2 text-sm text-white shadow-sm hover:border-white/20 transition-colors"
									>
										Sign In
									</a>
								)}
							</div>
						}
						mobileBreadcrumbs={
							<DocsMobileNavigation
								tree={mobileSidebar}
								sidebarData={sidebarData}
								isLightTheme={isLightTheme}
							/>
						}
						sheetClassName={isLightTheme ? "!bg-white [&>button]:!bg-white [&>button]:!text-zinc-900 [&>button]:!border-zinc-200" : undefined}
						lightTheme={isLightTheme}
						breadcrumbs={
							<div className={cn("flex items-center font-v2 subpixel-antialiased", isLightTheme && "[&_a]:!text-zinc-600 [&_button]:!text-zinc-600")}>
								{!isLightTheme && <ProductsDropdown active={active === "product"} />}
								<TextNavItem
									href={isLightTheme ? AGENT_OS_DOCS_HREF : "/docs"}
									ariaCurrent={active === "docs" ? "page" : undefined}
								>
									Documentation
								</TextNavItem>
								{isLightTheme && (
									<>
										<TextNavItem href="/agent-os/use-cases">
											Use Cases
										</TextNavItem>
										<TextNavItem
											href={AGENT_OS_REGISTRY_HREF}
											ariaCurrent={isRegistryPage ? "page" : undefined}
										>
											<span className="inline-flex items-center gap-2">
												<span>Registry</span>
												<span className="inline-flex min-w-6 items-center justify-center rounded-full border border-zinc-200 bg-zinc-100 px-1.5 py-0.5 text-[10px] font-medium leading-none text-zinc-600">
													{REGISTRY_PACKAGE_COUNT}
												</span>
											</span>
										</TextNavItem>
										<TextNavItem href="/agent-os/pricing">
											Pricing
										</TextNavItem>
									</>
								)}
								{!isLightTheme && (
									<TextNavItem
										href="/cloud"
										ariaCurrent={active === "pricing" ? "page" : undefined}
									>
										Pricing
									</TextNavItem>
								)}
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
				"sticky top-0 z-50 bg-neutral-950/80 backdrop-blur-lg",
				"[&>div:first-child]:px-3 md:[&>div:first-child]:max-w-none md:[&>div:first-child]:px-0 md:px-8",
				// 0 padding on bottom for larger screens when subnav is showing
				effectiveSubnav ? "pb-2 md:pb-0 md:pt-4" : "md:py-4",
				// Learn mode styling
				learnMode && "bg-[#1c1917] border-b border-[#44403c]",
			)}
			logo={
				<div className="hidden md:block">
					<LogoContextMenu>
						<a href="/">
							<img
								src={logoUrl.src}
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
							<HeaderSearch />
						</div>
					)}
					<RivetHeader.NavItem asChild className="p-2 mr-4">
						<a href="https://rivet.dev/discord" className="text-white/90">
							<Icon icon={faDiscord} className="drop-shadow-md" />
						</a>
					</RivetHeader.NavItem>
					<GitHubDropdown className="inline-flex items-center justify-center whitespace-nowrap rounded-md border border-white/10 px-4 py-2 h-10 text-sm mr-2 hover:border-white/20 text-white/90 hover:text-white transition-colors" />
					<a
						href="https://dashboard.rivet.dev"
						className="font-v2 subpixel-antialiased inline-flex items-center justify-center whitespace-nowrap rounded-md border border-white/10 bg-white/5 px-4 py-2 text-sm text-white shadow-sm hover:border-white/20 transition-colors"
					>
						Sign In
					</a>
				</div>
			}
			mobileBreadcrumbs={<DocsMobileNavigation tree={mobileSidebar} sidebarData={sidebarData} />}
			breadcrumbs={
				<div className="flex items-center font-v2 subpixel-antialiased">
					<ProductsDropdown active={active === "product"} />
					<TextNavItem
						href="/docs"
						ariaCurrent={active === "docs" ? "page" : undefined}
					>
						Documentation
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

	const mainLinks = isLightTheme
		? [
			{ href: AGENT_OS_DOCS_HREF, label: "Documentation" },
			{ href: "/agent-os/use-cases", label: "Use Cases" },
			{ href: AGENT_OS_REGISTRY_HREF, label: `Registry (${REGISTRY_PACKAGE_COUNT})` },
			{ href: "/agent-os/pricing", label: "Pricing" },
		]
		: [
			{ href: "/docs", label: "Documentation" },
			{ href: "/cloud", label: "Pricing" },
		];

	const products = [
		{ label: "agentOS", href: "/agent-os", logo: agentosLogoUrl },
		{ label: "Actors", href: "/actors", logo: actorsLogoUrl },
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
				{/* Home logo with agentOS */}
				<a href="/" className="py-3 px-2 flex items-center gap-2">
					<img
						src={logoIconWhiteUrl.src}
						alt="Rivet"
						width={24}
						height={24}
						className="h-6 w-6"
					/>
					<div className="h-4 w-px bg-zinc-300" />
					<img
						src="/images/agent-os/agentos-hero-logo.svg"
						className="h-4 w-auto"
						alt="agentOS"
					/>
				</a>

				{/* Main navigation links */}
				{mainLinks.map(({ href, label }) => (
					<a
						key={href}
						href={href}
						className="text-zinc-900 py-2 px-2 hover:bg-zinc-100 rounded-sm transition-colors"
					>
						{label}
					</a>
				))}

				{/* Install button */}
				<div className="mt-4 pt-4 border-t border-zinc-200">
					<a
						href="/install"
						className="flex items-center justify-center w-full rounded-md bg-zinc-900 px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-zinc-700"
					>
						Install
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
