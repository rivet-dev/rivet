"use client";

import { useState, useRef, useEffect } from "react";
import { Icon, faChevronDown, faEllipsis } from "@rivet-gg/icons";
import { deployOptions } from "@/data/deploy/shared";
import Link from "next/link";

export function DeployDropdown() {
	const [isOpen, setIsOpen] = useState(false);
	const dropdownRef = useRef<HTMLDivElement>(null);

	useEffect(() => {
		function handleClickOutside(event: MouseEvent) {
			if (dropdownRef.current && !dropdownRef.current.contains(event.target as Node)) {
				setIsOpen(false);
			}
		}

		if (isOpen) {
			document.addEventListener("mousedown", handleClickOutside);
		}

		return () => {
			document.removeEventListener("mousedown", handleClickOutside);
		};
	}, [isOpen]);

	const otherPlatforms = deployOptions.filter(
		option => option.title !== "Vercel" && option.title !== "Railway"
	);

	return (
		<div className="relative" ref={dropdownRef}>
			<button
				onClick={() => setIsOpen(!isOpen)}
				className="flex items-center gap-2 w-full rounded-md border border-white/10 bg-white/5 px-4 py-2 text-sm text-white hover:border-white/20 transition-colors"
			>
				<Icon icon={faEllipsis} className="text-sm" />
				More Platforms
				<Icon icon={faChevronDown} className="text-xs ml-auto" />
			</button>

			{isOpen && (
				<div className="absolute z-10 mt-2 w-full rounded-md border border-white/10 bg-zinc-900 shadow-lg">
					<div className="py-1">
						{otherPlatforms.map((option) => (
							<Link
								key={option.title}
								href={option.href}
								target="_blank"
								rel="noopener noreferrer"
								className="flex items-center gap-2 px-4 py-2 text-sm text-zinc-300 hover:bg-white/5 hover:text-white transition-colors"
								onClick={() => setIsOpen(false)}
							>
								{option.icon && <Icon icon={option.icon} className="text-sm" />}
								{option.shortTitle || option.title}
							</Link>
						))}
					</div>
				</div>
			)}
		</div>
	);
}
