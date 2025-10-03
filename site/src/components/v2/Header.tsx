"use client";
import { ActiveLink } from "@/components/ActiveLink";
import logoUrl from "@/images/rivet-logos/icon-text-white.svg";
import { cn } from "@rivet-gg/components";
import { Header as RivetHeader } from "@rivet-gg/components/header";
import { Icon, faDiscord } from "@rivet-gg/icons";
import Image from "next/image";
import Link from "next/link";
import { type ReactNode, useEffect, useState } from "react";
import { usePathname } from "next/navigation";
import { GitHubDropdown } from "./GitHubDropdown";
import { HeaderSearch } from "./HeaderSearch";
import { LogoContextMenu } from "./LogoContextMenu";

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
				<Link
					href={href}
					className="text-white"
					aria-current={ariaCurrent}
				>
					{children}
				</Link>
			</RivetHeader.NavItem>
		</div>
	);
}

interface HeaderProps {
	active?: "product" | "docs" | "blog" | "cloud" | "pricing" | "solutions";
	subnav?: ReactNode;
	mobileSidebar?: ReactNode;
	variant?: "floating" | "full-width";
}

export function Header({
	active,
	subnav,
	mobileSidebar,
	variant = "full-width",
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
							<LogoContextMenu>
								<Link href="/">
									<Image
										src={logoUrl.src || logoUrl}
										width={80}
										height={24}
										className="ml-1 w-20"
										alt="Rivet logo"
										unoptimized
									/>
								</Link>
							</LogoContextMenu>
						}
						subnav={subnav}
						support={
							<div className="flex flex-col gap-4 font-v2 subpixel-antialiased">
								<RivetHeader.NavItem asChild>
									<Link href="https://hub.rivet.gg">
										Sign In
									</Link>
								</RivetHeader.NavItem>
								<RivetHeader.NavItem asChild>
									<Link href="https://rivet.gg/discord">Discord</Link>
								</RivetHeader.NavItem>
								<RivetHeader.NavItem asChild>
									<Link href="/support">Support</Link>
								</RivetHeader.NavItem>
							</div>
						}
						links={
							<div className="flex flex-row items-center">
								{variant === "full-width" && <HeaderSearch />}
								<RivetHeader.NavItem
									asChild
									className="p-2 mr-4"
								>
									<Link
										href="https://rivet.gg/discord"
										className="text-white/90"
									>
										<Icon
											icon={faDiscord}
											className="drop-shadow-md"
										/>
									</Link>
								</RivetHeader.NavItem>
								<GitHubDropdown className="inline-flex items-center justify-center whitespace-nowrap rounded-md border border-white/10 px-4 py-2 h-10 text-sm mr-2 hover:border-white/20 text-white/90 hover:text-white transition-colors" />
								<Link
									href="https://hub.rivet.gg"
									className="font-v2 subpixel-antialiased inline-flex items-center justify-center whitespace-nowrap rounded-md border border-white/10 bg-white/5 px-4 py-2 text-sm text-white shadow-sm hover:border-white/20 transition-colors"
								>
									Sign In
								</Link>
							</div>
						}
						mobileBreadcrumbs={
							<DocsMobileNavigation tree={mobileSidebar} />
						}
						breadcrumbs={
							<div className="flex items-center font-v2 subpixel-antialiased">
						<TextNavItem
							href="/docs"
							ariaCurrent={
								active === "docs" ? "page" : undefined
							}
						>
							Documentation
						</TextNavItem>
						<TextNavItem
							href="/pricing"
							ariaCurrent={
								active === "pricing" ? "page" : undefined
							}
						>
							Pricing
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
				"[&>div:first-child]:px-3 md:[&>div:first-child]:max-w-none md:[&>div:first-child]:px-0 md:px-8",
				// 0 padding on bottom for larger screens when subnav is showing
				subnav ? "pb-2 md:pb-0 md:pt-4" : "md:py-4",
			)}
			logo={
				<LogoContextMenu>
					<Link href="/">
						<Image
							src={logoUrl.src || logoUrl}
							width={80}
							height={24}
							className="ml-1 w-20"
							alt="Rivet logo"
							unoptimized
						/>
					</Link>
				</LogoContextMenu>
			}
			subnav={subnav}
			support={
				<div className="flex flex-col gap-4 font-v2 subpixel-antialiased">
					<RivetHeader.NavItem asChild>
						<Link href="https://hub.rivet.gg">Sign In</Link>
					</RivetHeader.NavItem>
					<RivetHeader.NavItem asChild>
						<Link href="/discord">Discord</Link>
					</RivetHeader.NavItem>
					<RivetHeader.NavItem asChild>
						<Link href="/support">Support</Link>
					</RivetHeader.NavItem>
				</div>
			}
			links={
				<div className="flex flex-row items-center">
					<div className="mr-4">
						<HeaderSearch />
					</div>
					<RivetHeader.NavItem asChild className="p-2 mr-4">
						<Link href="https://rivet.gg/discord" className="text-white/90">
							<Icon icon={faDiscord} className="drop-shadow-md" />
						</Link>
					</RivetHeader.NavItem>
					<GitHubDropdown className="inline-flex items-center justify-center whitespace-nowrap rounded-md border border-white/10 px-4 py-2 h-10 text-sm mr-2 hover:border-white/20 text-white/90 hover:text-white transition-colors" />
					<Link
						href="https://hub.rivet.gg"
						className="font-v2 subpixel-antialiased inline-flex items-center justify-center whitespace-nowrap rounded-md border border-white/10 bg-white/5 px-4 py-2 text-sm text-white shadow-sm hover:border-white/20 transition-colors"
					>
						Sign In
					</Link>
				</div>
			}
			mobileBreadcrumbs={<DocsMobileNavigation tree={mobileSidebar} />}
			breadcrumbs={
				<div className="flex items-center font-v2 subpixel-antialiased">
				<TextNavItem
					href="/docs"
					ariaCurrent={active === "docs" ? "page" : undefined}
				>
					Documentation
				</TextNavItem>
				<TextNavItem
					href="/pricing"
					ariaCurrent={active === "pricing" ? "page" : undefined}
				>
					Pricing
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

	const mainNavLinks = [
		{ href: "/pricing", label: "Pricing" },
		{ href: "/changelog", label: "Changelog" },
		{ href: "/docs", label: "Documentation" },
	];

	const docsSubLinks = [
		{ href: "/docs/actors", label: "Actors" },
		{ href: "/docs/integrations", label: "Integrations" },
		{ href: "/docs/general", label: "Reference" },
		{ href: "/docs/self-hosting", label: "Self-Hosting" },
		{ href: "/docs/cloud", label: "Enterprise Cloud" },
	];

	// Create a filtered tree that excludes the problematic section headers
	const filteredTree = tree ? (
		<div className="[&_.text-muted-foreground]:hidden">
			{tree}
		</div>
	) : null;

	return (
		<div className="flex flex-col gap-4 font-v2 subpixel-antialiased">
			{/* Main navigation */}
			{mainNavLinks.map(({ href, label }) => (
				<RivetHeader.NavItem key={href} asChild>
					<ActiveLink href={href}>
						{label}
					</ActiveLink>
				</RivetHeader.NavItem>
			))}

			{/* Documentation sub-links */}
			<div className="">
				{docsSubLinks.map(({ href, label }) => (
					<div key={href} className="ml-2">
						<RivetHeader.NavItem asChild>
							<ActiveLink href={href}>
								{label}
							</ActiveLink>
						</RivetHeader.NavItem>
					</div>
				))}
			</div>
		</div>
	);
}
