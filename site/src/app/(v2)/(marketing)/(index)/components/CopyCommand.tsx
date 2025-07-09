"use client";

import { useState } from "react";
import { Icon, faArrowRight, faCopy, faCheck } from "@rivet-gg/icons";
import clsx from "clsx";

interface CopyCommandProps {
	children?: string;
	command?: string;
	className?: string;
}

export function CopyCommand({
	children,
	command,
	className,
}: CopyCommandProps) {
	const [copied, setCopied] = useState(false);

	const handleCopy = () => {
		const commandText = command || children || "";
		const textToCopy = commandText.startsWith("$")
			? commandText.substring(1).trim()
			: commandText;

		navigator.clipboard.writeText(textToCopy);

		setCopied(true);
		setTimeout(() => {
			setCopied(false);
		}, 1000);
	};

	return (
		<div
			className={clsx(
				"inline-flex items-center gap-2 px-3 py-2 rounded-lg transition-all duration-200 cursor-pointer group max-w-fit",
				className,
			)}
			onClick={handleCopy}
		>
			<div className="flex items-center justify-center w-6 h-6 text-white/40 group-hover:text-white/80 transition-colors duration-200">
				<Icon icon={faArrowRight} className="w-4 h-4" />
			</div>

			<div className="text-white/40 group-hover:text-white/80 font-mono text-sm font-medium transition-colors duration-200">
				{command || children}
			</div>

			<div className="relative w-5 h-5 flex items-center justify-center">
				<div
					className={`absolute transition-opacity duration-200 ${copied ? "opacity-0" : "opacity-0 group-hover:opacity-100"}`}
				>
					<Icon
						icon={faCopy}
						className="w-4 h-4 text-white/40 group-hover:text-white/80 transition-colors"
					/>
				</div>
				<div
					className={`absolute transition-opacity duration-200 ${copied ? "opacity-100" : "opacity-0"}`}
				>
					<Icon icon={faCheck} className="w-4 h-4 text-green-400" />
				</div>
			</div>
		</div>
	);
}

