import type { ReactNode } from 'react';
import { Eyebrow } from './Eyebrow';

// Porcelain catalog entry: hairline border, translucent white fill over the
// porcelain field, optional numbered eyebrow, pine footer link. Hook-free.
interface CatalogCardProps {
	title?: ReactNode;
	eyebrow?: ReactNode;
	index?: string;
	href?: string;
	linkLabel?: ReactNode;
	children?: ReactNode;
	className?: string;
}

export const CatalogCard = ({
	title,
	eyebrow,
	index,
	href,
	linkLabel,
	children,
	className,
}: CatalogCardProps) => {
	const body = (
		<>
			{eyebrow ? <Eyebrow label={eyebrow} index={index} /> : null}
			{title ? (
				<h3 className={`text-lg font-medium tracking-[-0.01em] text-ink md:text-xl ${eyebrow ? 'mt-4' : ''}`}>
					{title}
				</h3>
			) : null}
			{children}
			{href && linkLabel ? (
				<span className="mt-5 inline-flex items-center gap-1.5 text-sm font-medium text-pine">
					{linkLabel}
					<span aria-hidden="true" className="transition-transform duration-200 group-hover:translate-x-0.5">
						→
					</span>
				</span>
			) : null}
		</>
	);

	const cardClass = `group block border border-ink/10 bg-white/55 p-7 transition-colors duration-200 ${
		href ? 'hover:border-ink/25' : ''
	} ${className ?? ''}`;

	return href ? (
		<a href={href} className={cardClass}>
			{body}
		</a>
	) : (
		<div className={cardClass}>{body}</div>
	);
};
