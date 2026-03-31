import clsx from "clsx";
import type { PropsWithChildren } from "react";
import { Icon, faArrowRight } from "@rivet-gg/icons";

interface CardProps extends PropsWithChildren<{ className?: string }> {
	title?: string;
	icon?: any;
	href?: string;
	target?: string;
	badge?: string;
}

export function Card({
	children,
	className,
	title,
	icon,
	href,
	target,
	badge,
}: CardProps) {
	const hasHeader = Boolean(title || icon || href);
	const hasBody = Boolean(children);

	const content = (
		<div
			className={clsx(
				"rounded-xl bg-neutral-950 border border-white/20 shadow-sm transition-all duration-200 relative overflow-hidden flex flex-col w-full h-full",
				href && "group-hover:border-[white]/40 cursor-pointer",
				className,
			)}
		>
			{hasHeader && (
				<div className={clsx("px-8 mt-6", !hasBody && "pb-6")}>
					<div
						className={clsx(
							"flex items-center justify-between text-white text-base",
							hasBody && "mb-4",
						)}
					>
						<div className="flex items-center gap-3">
							{icon && <Icon icon={icon} />}
							{title && <h3 className="font-medium">{title}</h3>}
							{badge && (
								<span className="text-xs font-medium px-2 py-0.5 rounded-full bg-white/10 text-white/60">
									{badge}
								</span>
							)}
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
			{hasBody && (
				<div className={clsx("px-8", hasHeader ? "pb-6" : "py-6")}>
					{children}
				</div>
			)}
		</div>
	);

	if (href) {
		return (
			<a href={href} className="flex group w-full" target={target}>
				{content}
			</a>
		);
	}

	return content;
}

export const CardGroup = ({ children }: PropsWithChildren) => {
	return (
		<div className="not-prose grid gap-4 md:grid-cols-2">{children}</div>
	);
};
