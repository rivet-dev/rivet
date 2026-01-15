"use client";

import { cn } from "@rivet-gg/components";
import { Icon, faChevronDown } from "@rivet-gg/icons";
import { motion } from "framer-motion";
import { useState } from "react";

export function Foldable({
	title = "Show child properties",
	closeTitle = "Hide child properties",
	children,
}) {
	const [isOpen, setIsOpen] = useState(false);
	return (
		<>
			<button
				type="button"
				className={cn(
					"mt-2 flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground transition-colors",
				)}
				onClick={() => setIsOpen((open) => !open)}
			>
				{isOpen ? closeTitle : title}
				<motion.span
					variants={{
						open: { rotateZ: 0 },
						closed: { rotateZ: "-90deg" },
					}}
					initial={false}
					animate={isOpen ? "open" : "closed"}
					transition={{ duration: 0.2 }}
					className="inline-block"
				>
					<Icon icon={faChevronDown} className="w-3 h-3" />
				</motion.span>
			</button>
			<motion.div
				className="mt-1 overflow-hidden"
				initial={false}
				variants={{
					open: { height: "auto", opacity: 1 },
					closed: { height: 0, opacity: 0 },
				}}
				animate={isOpen ? "open" : "closed"}
				transition={{
					opacity: isOpen ? { delay: 0.3 } : {},
					height: !isOpen ? { delay: 0.3 } : {},
					duration: 0.3,
				}}
			>
				{children}
			</motion.div>
		</>
	);
}
