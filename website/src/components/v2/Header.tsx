"use client";
import { usePathname } from "@/hooks/usePathname";
import { ActiveLink } from "@/components/ActiveLink";
import logoUrl from "@/images/rivet-logos/icon-text-white.svg";
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
import {
	Bot,
	Gamepad2,
	FileText,
	Workflow,
	ShoppingCart,
	Wand2,
	Network,
	Clock,
	Database,
	Globe,
} from "lucide-react";
import actorsLogoUrl from "@/images/products/actors-logo.svg";
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

function ProductsDropdown({ active }: { active?: boolean }) {
	const [isOpen, setIsOpen] = useState(false);
	const closeTimeoutRef = useRef<NodeJS.Timeout | null>(null);
	const isHoveringRef = useRef(false);

	const products = [
		{
			label: "Actors",
			href: "/docs/actors",
			logo: actorsLogoUrl,
			description: "Build stateful backends",
		},
		{
			label: "Sandbox Agent SDK",
			href: "https://sandboxagent.dev/",
			logo: sandboxAgentLogoUrl,
			description: "SDK for coding agents",
			external: true,
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
			className="px-2.5 py-2 opacity-60 hover:opacity-100 transition-all duration-200"
			onMouseEnter={handleMouseEnter}
			onMouseLeave={handleMouseLeave}
		>
			<DropdownMenu open={isOpen} onOpenChange={handleOpenChange} modal={false}>
				<DropdownMenuTrigger asChild>
					<RivetHeader.NavItem asChild>
						<button
							type="button"
							className={cn(
								"!text-white cursor-pointer flex items-center gap-1 relative",
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
					className="min-w-[280px] p-4 bg-black/95 backdrop-blur-lg border border-white/10 rounded-xl shadow-xl"
					onMouseEnter={handleMouseEnter}
					onMouseLeave={handleMouseLeave}
					sideOffset={0}
					alignOffset={0}
					side="bottom"
				>
					<div className="flex flex-col gap-1">
						{products.map((product) => (
							<a
								key={product.href}
								href={product.href}
								target={product.external ? "_blank" : undefined}
								rel={product.external ? "noopener noreferrer" : undefined}
								className="group flex items-center gap-3 p-3 rounded-lg hover:bg-white/5 transition-colors cursor-pointer"
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
									<div className="font-medium text-white text-sm group-hover:text-white transition-colors">
										{product.label}
									</div>
									<div className="text-xs text-zinc-400 group-hover:text-zinc-300 transition-colors leading-relaxed">
										{product.description}
									</div>
								</div>
							</a>
						))}
					</div>
				</DropdownMenuContent>
			</DropdownMenu>
		</div>
	);
}

function SolutionsDropdown({ active }: { active?: boolean }) {
	const [isOpen, setIsOpen] = useState(false);
	const closeTimeoutRef = useRef<NodeJS.Timeout | null>(null);
	const isHoveringRef = useRef(false);

	const solutions = [
		{
			label: "Agent Orchestration",
			href: "/solutions/agents",
			icon: Bot,
			description: "Build durable AI assistants",
		},
		{
			label: "Multiplayer Documents",
			href: "/solutions/collaborative-state",
			icon: FileText,
			description: "Real-time collaboration",
		},
		{
			label: "Workflows",
			href: "/solutions/workflows",
			icon: Workflow,
			description: "Durable multi-step processes",
		},
		{
			label: "Vibe-Coded Backends",
			href: "/solutions/app-generators",
			icon: Wand2,
			description: "Backend for AI-generated apps",
		},
		{
			label: "Geo-Distributed Databases",
			href: "/solutions/geo-distributed-db",
			icon: Globe,
			description: "Multi-region state replication",
		},
		{
			label: "Per-Tenant Databases",
			href: "/solutions/per-tenant-db",
			icon: Database,
			description: "Isolated state per customer",
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

	// Handle Radix's open change events (escape key, click outside, etc.)
	const handleOpenChange = (open: boolean) => {
		if (!open) {
			// Close immediately for keyboard/click-outside events
			// These fire when user explicitly wants to close
			cancelClose();
			setIsOpen(false);
		}
		// Ignore open events from Radix - hover controls opening
	};

	// Click toggles the menu, but we handle it ourselves to avoid Radix's double-toggle
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
			className="px-2.5 py-2 opacity-60 hover:opacity-100 transition-all duration-200"
			onMouseEnter={handleMouseEnter}
			onMouseLeave={handleMouseLeave}
		>
			<DropdownMenu open={isOpen} onOpenChange={handleOpenChange} modal={false}>
				<DropdownMenuTrigger asChild>
					<RivetHeader.NavItem asChild>
						<button
							type="button"
							className={cn(
								"!text-white cursor-pointer flex items-center gap-1 relative",
								active && "opacity-100",
								// Invisible bridge to prevent gap issues when moving to dropdown
								"after:absolute after:left-0 after:right-0 after:top-full after:h-4 after:content-['']",
							)}
							onPointerDown={handlePointerDown}
							onMouseEnter={handleMouseEnter}
						>
							Solutions
							<Icon icon={faChevronDown} className="h-3 w-3 ml-0.5" />
						</button>
					</RivetHeader.NavItem>
				</DropdownMenuTrigger>
				<DropdownMenuContent
					align="start"
					className="min-w-[600px] p-6 bg-black/95 backdrop-blur-lg border border-white/10 rounded-xl shadow-xl"
					onMouseEnter={handleMouseEnter}
					onMouseLeave={handleMouseLeave}
					sideOffset={0}
					alignOffset={0}
					side="bottom"
				>
					<div className="grid grid-cols-2 gap-x-8 gap-y-5">
						{solutions.map((solution) => {
							const IconComponent = solution.icon;
							return (
								<a
									key={solution.href}
									href={solution.href}
									className="group flex items-start gap-4 p-3 rounded-lg hover:bg-white/5 transition-colors cursor-pointer -m-3"
								>
									<div className="flex-shrink-0 pt-0.5 text-zinc-500 group-hover:text-zinc-400 transition-colors">
										<IconComponent className="w-4 h-4" />
									</div>
									<div className="flex-1 min-w-0 pt-0.5">
										<div className="font-medium text-white mb-1.5 text-sm group-hover:text-white transition-colors">
											{solution.label}
										</div>
										<div className="text-xs text-zinc-400 group-hover:text-zinc-300 transition-colors leading-relaxed">
											{solution.description}
										</div>
									</div>
								</a>
							);
						})}
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
	| "cloud"
	| "solutions"
	| "learn";
	subnav?: ReactNode;
	mobileSidebar?: ReactNode;
	variant?: "floating" | "full-width";
	learnMode?: boolean;
	showDocsTabs?: boolean;
}

export function Header({
	active,
	subnav,
	mobileSidebar,
	variant = "full-width",
	learnMode = false,
	showDocsTabs = false,
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

	if (variant === "floating") {
		const headerStyles = cn(
			"md:border-transparent md:static md:bg-transparent md:rounded-2xl md:max-w-[1200px] md:border-transparent md:backdrop-none [&>div:first-child]:px-3 md:backdrop-blur-none transition-all hover:opacity-100",
			isScrolled ? "opacity-100" : "opacity-80",
		);

		return (
			<div className="fixed top-0 z-50 w-full max-w-[1200px] md:left-1/2 md:top-4 md:-translate-x-1/2 md:px-8">
				<div
					className={cn(
						"hero-bg-exclude",
						'relative before:pointer-events-none before:absolute before:inset-[-1px] before:z-20 before:hidden before:rounded-2xl before:border before:border-white/10 before:content-[""] before:transition-colors before:duration-300 before:ease-in-out md:before:block',
					)}
				>
					<div className="absolute inset-0 -z-[1] hidden overflow-hidden rounded-2xl bg-background/80 backdrop-blur-lg md:block" />
					<RivetHeader
						className={headerStyles}
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
										/>
									</a>
								</LogoContextMenu>
							</div>
						}
						subnav={effectiveSubnav}
						support={null}
						links={
							<div className="flex flex-row items-center">
								{variant === "full-width" && <HeaderSearch />}
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
						mobileBreadcrumbs={<DocsMobileNavigation tree={mobileSidebar} />}
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
									href="/cookbook"
									ariaCurrent={active === "cookbook" ? "page" : undefined}
								>
									Cookbooks
								</TextNavItem>
								<TextNavItem
									href="/cloud"
									ariaCurrent={active === "cloud" ? "page" : undefined}
								>
									Cloud
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
			mobileBreadcrumbs={<DocsMobileNavigation tree={mobileSidebar} />}
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
						href="/cookbook"
						ariaCurrent={active === "cookbook" ? "page" : undefined}
					>
						Cookbooks
					</TextNavItem>
					<TextNavItem
						href="/cloud"
						ariaCurrent={active === "cloud" ? "page" : undefined}
					>
						Cloud
					</TextNavItem>
				</div>
			}
		/>
	);
}

function DocsMobileNavigation({ tree }) {
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
		{ href: "/cloud", label: "Cloud" },
		{ href: "/cookbook", label: "Cookbooks" },
	];

	const solutions = [
		{ label: "Agent Orchestration", href: "/solutions/agents", icon: Bot },
		{
			label: "Multiplayer Documents",
			href: "/solutions/collaborative-state",
			icon: FileText,
		},
		{ label: "Workflows", href: "/solutions/workflows", icon: Workflow },
		{
			label: "Vibe-Coded Backends",
			href: "/solutions/app-generators",
			icon: Wand2,
		},
		{
			label: "Geo-Distributed Databases",
			href: "/solutions/geo-distributed-db",
			icon: Globe,
		},
		{
			label: "Per-Tenant Databases",
			href: "/solutions/per-tenant-db",
			icon: Database,
		},
	];

	const products = [
		{ label: "Actors", href: "/docs/actors", logo: actorsLogoUrl },
		{
			label: "Sandbox Agent SDK",
			href: "https://sandboxagent.dev/",
			logo: sandboxAgentLogoUrl,
			external: true,
		},
	];

	const currentSection = sections.find((s) => s.id === getCurrentSection());

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
