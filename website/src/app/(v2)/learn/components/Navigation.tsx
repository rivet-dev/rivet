"use client";

import Link from "next/link";
import { Icon, faChevronLeft, faChevronRight, faBookOpen } from "@rivet-gg/icons";

interface NavigationProps {
	prevHref?: string;
	nextHref?: string;
	showPrev?: boolean;
	showNext?: boolean;
}

export function Navigation({
	prevHref,
	nextHref,
	showPrev = true,
	showNext = true,
}: NavigationProps) {
	return (
		<div className="fixed bottom-0 left-0 right-0 bg-[#1c1917] border-t border-[#44403c] p-4 z-50">
			<div className="max-w-3xl mx-auto flex justify-between items-center font-serif text-[#d4b483]">
				{showPrev && prevHref ? (
					<Link
						href={prevHref}
						className="flex items-center space-x-2 hover:text-[#e7e5e4] transition-opacity no-underline"
					>
						<Icon icon={faChevronLeft} className="w-5 h-5" />
						<span className="hidden md:inline">Previous Scene</span>
					</Link>
				) : (
					<div className="flex items-center space-x-2 opacity-20 cursor-not-allowed">
						<Icon icon={faChevronLeft} className="w-5 h-5" />
						<span className="hidden md:inline">Previous Scene</span>
					</div>
				)}

				<Link
					href="/learn"
					className="flex flex-col items-center group no-underline"
				>
					<span className="text-xs uppercase tracking-widest opacity-50 group-hover:opacity-100 transition-opacity">
						Table of Contents
					</span>
					<Icon icon={faBookOpen} className="w-5 h-5 mt-1" />
				</Link>

				{showNext && nextHref ? (
					<Link
						href={nextHref}
						className="flex items-center space-x-2 hover:text-[#e7e5e4] transition-opacity no-underline"
					>
						<span className="hidden md:inline">Next Scene</span>
						<Icon icon={faChevronRight} className="w-5 h-5" />
					</Link>
				) : (
					<div className="flex items-center space-x-2 opacity-20 cursor-not-allowed">
						<span className="hidden md:inline">Next Scene</span>
						<Icon icon={faChevronRight} className="w-5 h-5" />
					</div>
				)}
			</div>
		</div>
	);
}
