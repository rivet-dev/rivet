"use client";

import type { SidebarItem, SidebarSection } from "@/lib/sitemap";
import { Icon, faChevronDown } from "@rivet-gg/icons";
import { motion } from "framer-motion";
import { usePathname } from "next/navigation";
import { type ReactNode, useState } from "react";

interface CollapsibleSidebarItemProps {
	item: SidebarSection;
	children?: ReactNode;
}

export function CollapsibleSidebarItem({
	item,
	children,
}: CollapsibleSidebarItemProps) {
	const pathname = usePathname() || "";
	const isCurrent = findActiveItem(item.pages, pathname) !== null;
	const [isOpen, setIsOpen] = useState(() => isCurrent);
	return (
		<div>
			<button
				type="button"
				className="flex w-full appearance-none items-center px-2 py-1 text-sm text-muted-foreground transition-colors data-[active]:text-foreground"
				data-active={isCurrent ? true : undefined}
				onClick={() => setIsOpen((open) => !open)}
			>
				{item.icon ? (
					<Icon icon={item.icon} className="mr-2 size-3.5" />
				) : null}
				<span className="truncate">
					{item.title}
				</span>
				<motion.span
					variants={{
						open: { rotateZ: 0 },
						closed: { rotateZ: "-90deg" },
					}}
					initial={isCurrent ? "open" : "closed"}
					animate={isOpen ? "open" : "closed"}
					className="ml-2 mr-2 inline-block w-2.5"
				>
					<Icon icon={faChevronDown} className="size-auto" />
				</motion.span>
			</button>
			<motion.div
				className="overflow-hidden pl-3"
				initial={isCurrent ? "open" : "closed"}
				variants={{
					open: { height: "auto", opacity: 1 },
					closed: { height: 0, opacity: 0 },
				}}
				animate={isOpen ? "open" : "closed"}
				transition={{
					opacity: isOpen ? { delay: 0.1 } : {},
					height: !isOpen ? { delay: 0.1 } : {},
					duration: 0.2,
				}}
			>
				{children}
			</motion.div>
		</div>
	);
}

function findActiveItem(pages: SidebarItem[], href: string) {
	for (const page of pages) {
		if ("href" in page && page.href === href) {
			return page;
		}
		if ("pages" in page) {
			const found = findActiveItem(page.pages, href);
			if (found) {
				return found;
			}
		}
	}

	return null;
}
