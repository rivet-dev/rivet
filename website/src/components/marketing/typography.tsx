import type { ReactNode } from 'react';

// Canonical marketing typography. These class strings are the single source of
// truth for heading treatment on marketing surfaces. Use them (or SectionHeading)
// instead of hand-writing tracking and weight classes on new pages.

// Hero H1 on porcelain marketing pages.
export const HERO_H1_CLASS =
	'text-4xl font-medium leading-[1.06] tracking-[-0.015em] text-ink text-balance md:text-6xl';

// Section H2 on porcelain marketing pages.
export const SECTION_H2_CLASS =
	'text-3xl font-medium tracking-[-0.015em] text-ink md:text-4xl';

// Muted subtitle that sits under a hero or section heading.
export const SUBTITLE_CLASS = 'mt-4 text-base leading-relaxed text-ink-soft';

// Letter-spaced mono eyebrow, the museum-catalog section label. Pine on
// porcelain; use text-sage when the eyebrow sits inside an ink panel.
export const EYEBROW_CLASS =
	'font-mono text-[11px] font-medium uppercase tracking-[0.18em] text-pine';

// Printed catalog caption, e.g. "Fig. 02 — Rivet Inspector".
export const CAPTION_CLASS = 'font-mono text-xs text-ink-faint';

interface SectionHeadingProps {
	title: ReactNode;
	subtitle?: ReactNode;
	eyebrow?: ReactNode;
	index?: string;
	className?: string;
}

export const SectionHeading = ({ title, subtitle, eyebrow, index, className }: SectionHeadingProps) => (
	<div className={className}>
		{eyebrow ? (
			<p className={`${EYEBROW_CLASS} mb-4`}>
				{index ? <span className="text-ink-faint">{index} — </span> : null}
				{eyebrow}
			</p>
		) : null}
		<h2 className={SECTION_H2_CLASS}>{title}</h2>
		{subtitle ? <p className={SUBTITLE_CLASS}>{subtitle}</p> : null}
	</div>
);
