import type { ReactNode } from 'react';

// Canonical marketing typography. These class strings are the single source of
// truth for heading treatment on marketing surfaces. Use them (or SectionHeading)
// instead of hand-writing tracking and weight classes on new pages.

// Hero H1 on dark marketing pages.
export const HERO_H1_CLASS =
	'text-4xl font-medium leading-[1.06] tracking-[-0.015em] text-white md:text-6xl';

// Section H2 on dark marketing pages.
export const SECTION_H2_CLASS =
	'text-3xl font-medium tracking-[-0.015em] text-white md:text-4xl';

// Muted subtitle that sits under a hero or section heading.
export const SUBTITLE_CLASS = 'mt-4 text-base leading-relaxed text-zinc-500';

interface SectionHeadingProps {
	title: ReactNode;
	subtitle?: ReactNode;
	className?: string;
}

export const SectionHeading = ({ title, subtitle, className }: SectionHeadingProps) => (
	<div className={className}>
		<h2 className={SECTION_H2_CLASS}>{title}</h2>
		{subtitle ? <p className={SUBTITLE_CLASS}>{subtitle}</p> : null}
	</div>
);
