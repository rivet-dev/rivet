import clsx from "clsx";
import type { PropsWithChildren } from "react";
import { Icon, faArrowRight } from "@rivet-gg/icons";
import Link from "next/link";

interface CardProps extends PropsWithChildren<{ className?: string }> {
	title?: string;
	icon?: any;
	href?: string;
	target?: string;
}

export function Card({
	children,
	className,
	title,
	icon,
	href,
	target,
}: CardProps) {
	const content = (
		<div
			className={clsx(
				"rounded-xl bg-white/2 border border-white/20 shadow-sm transition-all duration-200 relative overflow-hidden flex flex-col",
				href && "group-hover:border-[white]/40 cursor-pointer",
				className,
			)}
		>
			{(title || icon || href) && (
				<div className="px-8 mt-6">
					<div className="flex items-center justify-between mb-4 text-white text-base">
						<div className="flex items-center gap-3">
							{icon && <Icon icon={icon} />}
							{title && <h3 className="font-medium">{title}</h3>}
						</div>
						{href && (
							<Icon
								icon={faArrowRight}
								className="text-sm text-white/40 group-hover:text-white transition-all duration-200 group-hover:translate-x-1"
							/>
						)}
					</div>
				</div>
			)}
			<div className={clsx("px-8", (title || icon) ? "pb-6" : "py-6")}>
				{children}
			</div>
		</div>
	);

	if (href) {
		return (
			<Link href={href} className="block group" target={target}>
				{content}
			</Link>
		);
	}

	return content;
}

export const CardGroup = ({ children }: PropsWithChildren) => {
	return (
		<div className="not-prose grid gap-4 md:grid-cols-2">{children}</div>
	);
};
