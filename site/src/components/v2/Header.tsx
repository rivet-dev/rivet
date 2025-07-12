"use client";
import { DocsMobileNavigation } from "@/components/DocsMobileNavigation";
import logoUrl from "@/images/rivet-logos/icon-text-white.svg";
import { Button } from "@rivet-gg/components";
import { Header as RivetHeader } from "@rivet-gg/components/header";
import { Icon, faDiscord } from "@rivet-gg/icons";
import Image from "next/image";
import Link from "next/link";
import { type ReactNode, useState } from "react";
import { GitHubDropdown } from "./GitHubDropdown";
import { HeaderSearch } from "./HeaderSearch";

interface HeaderProps {
	active?: "product" | "docs" | "blog" | "cloud" | "pricing" | "solutions";
	subnav?: ReactNode;
	mobileBreadcrumbs?: ReactNode;
}

export function Header({ active, subnav }: HeaderProps) {
	const [ref, setRef] = useState<Element | null>(null);
	return (
		<RivetHeader
			className="lg:px-8 md:[&>div:first-child]:max-w-[calc(20rem+65ch+20rem)] md:[&>div:first-child]:px-0"
			logo={
				<Link href="/">
					<Image {...logoUrl} className="w-20" alt="Rivet logo" />
				</Link>
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
				<>
					<HeaderSearch />
					<RivetHeader.NavItem asChild className="-m-2 p-2">
						<Link href="/discord">
							<Icon icon={faDiscord} />
						</Link>
					</RivetHeader.NavItem>
					<GitHubDropdown className="p-2" />
					<Button
						variant="outline"
						asChild
						className="font-v2 text-foreground subpixel-antialiased"
					>
						<Link href="https://hub.rivet.gg">Sign In</Link>
					</Button>
				</>
			}
			mobileBreadcrumbs={<DocsMobileNavigation />}
			breadcrumbs={
				<div className="flex items-center font-v2 subpixel-antialiased">
					<RivetHeader.NavItem
						asChild
						className="flex items-center gap-1 px-2.5 py-2 first:pl-0"
					>
						<Link
							href="/docs"
							aria-current={
								active === "docs" ? "page" : undefined
							}
						>
							Documentation
						</Link>
					</RivetHeader.NavItem>
					<RivetHeader.NavItem
						asChild
						className="flex items-center gap-1 px-2.5 py-2"
					>
						<Link
							href="/cloud"
							aria-current={
								active === "cloud" ? "page" : undefined
							}
						>
							Cloud
						</Link>
					</RivetHeader.NavItem>
					<RivetHeader.NavItem
						asChild
						className="flex items-center gap-1 px-2.5 py-2"
					>
						<Link
							href="/changelog"
							aria-current={
								active === "blog" ? "page" : undefined
							}
						>
							Changelog
						</Link>
					</RivetHeader.NavItem>
				</div>
			}
		/>
	);
}
