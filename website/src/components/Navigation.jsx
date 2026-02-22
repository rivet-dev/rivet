"use client";
import { usePathname } from "@/hooks/usePathname";
import routes from "@/generated/routes.json";
import { faBooks, faCoin, faNewspaper } from "@rivet-gg/icons";
import clsx from "clsx";
import { motion } from "framer-motion";

import { Button } from "@/components/Button";
import { useIsInsideMobileNavigation } from "@/components/MobileNavigation";
import { ActiveSectionMarker } from "@/components/TableOfContents";
import { Tag } from "@/components/Tag";
import { normalizePath } from "@/lib/normalizePath";

function TopLevelNavItem({ href, target, children }) {
	return (
		<li className="lg:hidden">
			<a href={href}
				target={target}
				className="block py-1 text-sm text-charcole-400 transition hover:text-white"
			>
				{children}
			</a>
		</li>
	);
}

function NavLink({ href, tag, active, isAnchorLink = false, children }) {
	return (
		<a href={href}
			aria-current={active ? "page" : undefined}
			className={clsx(
				"flex justify-between gap-2 py-1 pr-3 text-sm transition",
				isAnchorLink ? "pl-7" : "pl-4",
				active ? "text-white" : "text-charcole-400 hover:text-white",
			)}
		>
			<span className="truncate">{children}</span>
			{tag && (
				<Tag variant="small" color="zinc">
					{tag}
				</Tag>
			)}
		</a>
	);
}

function NavigationGroup({ group, className }) {
	// If this is the mobile navigation then we always render the initial
	// state, so that the state does not change during the close animation.
	// The state will still update when we re-open (re-render) the navigation.
	const isInsideMobileNavigation = useIsInsideMobileNavigation();
	const pathname = usePathname();

	return (
		<li className={clsx("relative mt-6", className)}>
			<motion.h2
				layout="position"
				className="font-sans text-xs font-semibold text-white"
			>
				{group.title}
			</motion.h2>
			<div className="relative mt-3 pl-2">
				<motion.div
					layout
					className="absolute inset-y-0 left-2 w-px bg-white/5"
				/>
				<ul role="list" className="border-l border-transparent">
					{group.pages.map((link) => {
						const page = routes.pages[link.href];
						return (
							<motion.li
								key={link.href}
								layout="position"
								className="relative"
							>
								{normalizePath(link.href) === normalizePath(pathname) ? (
									<ActiveSectionMarker prefix="navigation" />
								) : null}
								<NavLink
									href={link.href}
									active={normalizePath(link.href) === normalizePath(pathname)}
								>
									{page.title}
								</NavLink>
							</motion.li>
						);
					})}
				</ul>
			</div>
		</li>
	);
}

export function Navigation({ navigation, ...props }) {
	const overviewGroup = {
		title: "",
		pages: [
			{ title: "Overview", href: navigation.prefix },
			...(navigation.pages ?? []),
		],
	};

	return (
		<nav {...props}>
			<ul role="list">
				{/* Header */}
				<TopLevelNavItem href="/docs" icon={faBooks}>
					Documentation
				</TopLevelNavItem>
				<TopLevelNavItem href="/changelog" icon={faNewspaper}>
					Changelog
				</TopLevelNavItem>
				<TopLevelNavItem href="/cloud" icon={faCoin}>
					Cloud
				</TopLevelNavItem>

				{/* Sidebar */}
				{navigation.sidebar
					? [overviewGroup, ...navigation.sidebar.groups].map(
							(group, groupIndex) => (
								<NavigationGroup
									key={group.title}
									group={group}
									className={groupIndex === 0 && "md:mt-6"}
								/>
							),
						)
					: null}
				<li className="sticky bottom-0 z-10 mt-6 min-[416px]:hidden">
					<Button
						href="https://dashboard.rivet.dev"
						variant="secondary"
						className="w-full"
					>
						Sign In
					</Button>
				</li>
			</ul>
		</nav>
	);
}
