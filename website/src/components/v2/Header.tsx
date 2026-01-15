"use client";
import { usePathname } from "@/hooks/usePathname";
import { ActiveLink } from "@/components/ActiveLink";
import logoUrl from "@/images/rivet-logos/icon-text-white.svg";
import { cn } from "@rivet-gg/components";
import { Header as RivetHeader } from "@rivet-gg/components/header";
import { Icon, faDiscord } from "@rivet-gg/icons";
import React, { type ReactNode, useEffect, useRef, useState } from "react";
import { Button, DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "@rivet-gg/components";
import { faChevronDown } from "@rivet-gg/icons";
import { Bot, Gamepad2, FileText, Workflow, ShoppingCart, Wand2, Network, Clock, Database, Globe } from "lucide-react";
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
				<a href={href}
					className="text-white"
					aria-current={ariaCurrent}
				>
					{children}
				</a>
			</RivetHeader.NavItem>
		</div>
	);
}

function SolutionsDropdown({ active }: { active?: boolean }) {
	const [isOpen, setIsOpen] = useState(false);
	const closeTimeoutRef = useRef<NodeJS.Timeout | null>(null);

	const solutions = [
		{ 
			label: "Agent Orchestration", 
			href: "/solutions/agents",
			icon: Bot,
			description: "Build durable AI assistants"
		},
		{ 
			label: "Multiplayer Documents", 
			href: "/solutions/collaborative-state",
			icon: FileText,
			description: "Real-time collaboration"
		},
		{ 
			label: "Workflows", 
			href: "/solutions/workflows",
			icon: Workflow,
			description: "Durable multi-step processes"
		},
		{ 
			label: "Vibe-Coded Backends", 
			href: "/solutions/app-generators",
			icon: Wand2,
			description: "Backend for AI-generated apps"
		},
		{ 
			label: "Geo-Distributed Databases", 
			href: "/solutions/geo-distributed-db",
			icon: Globe,
			description: "Multi-region state replication"
		},
		{ 
			label: "Per-Tenant Databases", 
			href: "/solutions/per-tenant-db",
			icon: Database,
			description: "Isolated state per customer"
		},
	];

	const handleMouseEnter = () => {
		if (closeTimeoutRef.current) {
			clearTimeout(closeTimeoutRef.current);
			closeTimeoutRef.current = null;
		}
		setIsOpen(true);
	};

	const handleMouseLeave = () => {
		closeTimeoutRef.current = setTimeout(() => {
			setIsOpen(false);
		}, 200);
	};

	const handleClick = (e: React.MouseEvent<HTMLElement>) => {
		e.preventDefault();
		e.stopPropagation();
		if (closeTimeoutRef.current) {
			clearTimeout(closeTimeoutRef.current);
			closeTimeoutRef.current = null;
		}
		setIsOpen(!isOpen);
	};

	const handleOpenChange = (open: boolean) => {
		if (closeTimeoutRef.current) {
			clearTimeout(closeTimeoutRef.current);
			closeTimeoutRef.current = null;
		}
		setIsOpen(open);
	};

	useEffect(() => {
		return () => {
			if (closeTimeoutRef.current) {
				clearTimeout(closeTimeoutRef.current);
			}
		};
	}, []);

	return (
		<div 
			className="px-2.5 py-2 opacity-60 hover:opacity-100 transition-all duration-200"
			onMouseEnter={handleMouseEnter}
			onMouseLeave={handleMouseLeave}
		>
			<DropdownMenu open={isOpen} onOpenChange={handleOpenChange} modal={false}>
				<DropdownMenuTrigger asChild>
					<RivetHeader.NavItem
						className={cn(
							"!text-white cursor-pointer flex items-center gap-1 relative",
							active && "opacity-100",
							// Invisible bridge to prevent gap issues
							"after:absolute after:left-0 after:right-0 after:top-full after:h-4 after:content-['']",
						)}
						onClick={handleClick}
					>
						Solutions
						<Icon icon={faChevronDown} className="h-3 w-3 ml-0.5" />
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
									<div className="flex-shrink-0 w-10 h-10 rounded-lg border border-white/10 bg-white/5 flex items-center justify-center group-hover:border-white/20 group-hover:bg-white/10 transition-colors">
										<IconComponent className="w-5 h-5 text-white" />
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
	active?: "product" | "docs" | "blog" | "cloud" | "solutions" | "learn";
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
						'relative before:pointer-events-none before:absolute before:inset-[-1px] before:z-20 before:hidden before:rounded-2xl before:border before:content-[""] before:transition-colors before:duration-300 before:ease-in-out md:before:block',
						isScrolled
							? "before:border-white/10"
							: "before:border-transparent",
					)}
				>
					<div
						className={cn(
							"absolute inset-0 -z-[1] hidden overflow-hidden rounded-2xl transition-all duration-300 ease-in-out md:block",
							isScrolled
								? "bg-background/80 backdrop-blur-lg"
								: "bg-background backdrop-blur-none",
						)}
					/>
					<RivetHeader
						className={headerStyles}
						logo={
							<div className="hidden md:block">
								<LogoContextMenu>
									<a href="/">
										<img src={logoUrl.src}
											width={80}
											height={24}
											className="ml-1 w-20"
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
								<RivetHeader.NavItem
									asChild
									className="p-2 mr-4"
								>
									<a href="https://rivet.dev/discord"
										className="text-white/90"
									>
										<Icon
											icon={faDiscord}
											className="drop-shadow-md"
										/>
									</a>
								</RivetHeader.NavItem>
								<GitHubDropdown className="inline-flex items-center justify-center whitespace-nowrap rounded-md border border-white/10 px-4 py-2 h-10 text-sm mr-2 hover:border-white/20 text-white/90 hover:text-white transition-colors" />
								<a href="https://dashboard.rivet.dev"
									className="font-v2 subpixel-antialiased inline-flex items-center justify-center whitespace-nowrap rounded-md border border-white/10 bg-white/5 px-4 py-2 text-sm text-white shadow-sm hover:border-white/20 transition-colors"
								>
									Sign In
								</a>
							</div>
						}
						mobileBreadcrumbs={
							<DocsMobileNavigation tree={mobileSidebar} />
						}
						breadcrumbs={
							<div className="flex items-center font-v2 subpixel-antialiased">
								<SolutionsDropdown active={active === "solutions"} />
								<TextNavItem
									href="/docs"
									ariaCurrent={
										active === "docs" ? "page" : undefined
									}
								>
									Docs
								</TextNavItem>
								<TextNavItem
									href="/templates"
									ariaCurrent={
										active === "templates" ? "page" : undefined
									}
								>
									Templates
								</TextNavItem>
								<TextNavItem
									href="/cloud"
									ariaCurrent={
										active === "cloud" ? "page" : undefined
									}
								>
									Cloud
								</TextNavItem>
								<TextNavItem
									href="/changelog"
									ariaCurrent={
										active === "blog" ? "page" : undefined
									}
								>
									Changelog
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
				"sticky top-0 z-50 bg-neutral-950",
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
							<img src={logoUrl.src}
								width={80}
								height={24}
								className="ml-1 w-20"
								alt="Rivet logo"
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
					<a href="https://dashboard.rivet.dev"
						className="font-v2 subpixel-antialiased inline-flex items-center justify-center whitespace-nowrap rounded-md border border-white/10 bg-white/5 px-4 py-2 text-sm text-white shadow-sm hover:border-white/20 transition-colors"
					>
						Sign In
					</a>
				</div>
			}
			mobileBreadcrumbs={<DocsMobileNavigation tree={mobileSidebar} />}
			breadcrumbs={
				<div className="flex items-center font-v2 subpixel-antialiased">
					<SolutionsDropdown active={active === "solutions"} />
					<TextNavItem
						href="/docs"
						ariaCurrent={active === "docs" ? "page" : undefined}
					>
						Docs
					</TextNavItem>
					<TextNavItem
						href="/templates"
						ariaCurrent={active === "templates" ? "page" : undefined}
					>
						Templates
					</TextNavItem>
					<TextNavItem
						href="/cloud"
						ariaCurrent={active === "cloud" ? "page" : undefined}
					>
						Cloud
					</TextNavItem>
					<TextNavItem
						href="/changelog"
						ariaCurrent={active === "blog" ? "page" : undefined}
					>
						Changelog
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
		{ href: "/", label: "Home" },
		{ href: "/docs", label: "Docs" },
		{ href: "/templates", label: "Templates" },
		{ href: "/changelog", label: "Changelog" },
		{ href: "/cloud", label: "Cloud" },
		{ href: "https://dashboard.rivet.dev/", label: "Dashboard" },
	];

	const solutions = [
		{ label: "Agent Orchestration", href: "/solutions/agents", icon: Bot },
		{ label: "Multiplayer Documents", href: "/solutions/collaborative-state", icon: FileText },
		{ label: "Workflows", href: "/solutions/workflows", icon: Workflow },
		{ label: "Vibe-Coded Backends", href: "/solutions/app-generators", icon: Wand2 },
		{ label: "Geo-Distributed Databases", href: "/solutions/geo-distributed-db", icon: Globe },
		{ label: "Per-Tenant Databases", href: "/solutions/per-tenant-db", icon: Database },
	];

	const currentSection = sections.find(s => s.id === getCurrentSection());

	return (
		<div className="flex flex-col gap-1 font-v2 subpixel-antialiased text-sm">
			{/* Solutions dropdown */}
			<DropdownMenu>
				<DropdownMenuTrigger asChild>
					<button className="text-foreground py-1.5 px-2 hover:bg-accent rounded-sm transition-colors flex items-center justify-between w-full text-left">
						Solutions
						<Icon icon={faChevronDown} className="h-3 w-3" />
					</button>
				</DropdownMenuTrigger>
				<DropdownMenuContent 
					className="w-[calc(100%-2rem)] max-w-[292px]"
					align="start"
					sideOffset={4}
				>
					{solutions.map((solution) => {
						const IconComponent = solution.icon;
						return (
							<DropdownMenuItem
								key={solution.href}
								asChild
								className="flex items-center gap-2"
							>
								<a href={solution.href}>
									<IconComponent className="h-4 w-4" />
									{solution.label}
								</a>
							</DropdownMenuItem>
						);
					})}
				</DropdownMenuContent>
			</DropdownMenu>

			{/* Main navigation links */}
			{mainLinks.map(({ href, label }) => (
				<a key={href} href={href} className="text-foreground py-1.5 px-2 hover:bg-accent rounded-sm transition-colors">
					{label}
				</a>
			))}

			{/* Separator and docs content */}
			{isDocsPage && (
				<>
					<div className="border-t-2 border-border/50 my-2" />

					{/* Section dropdown */}
					<DropdownMenu>
						<DropdownMenuTrigger asChild>
							<Button variant="outline" className="w-full justify-between h-9 text-sm">
								{currentSection?.label || "Select Section"}
								<Icon icon={faChevronDown} className="h-3.5 w-3.5 ml-2" />
							</Button>
						</DropdownMenuTrigger>
						<DropdownMenuContent className="w-[calc(100vw-3rem)]">
							{sections.map(({ id, label, href }) => (
								<DropdownMenuItem
									key={id}
									asChild
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
		</div>
	);
}
