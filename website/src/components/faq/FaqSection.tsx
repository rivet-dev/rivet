'use client';

import { useState } from 'react';
import { motion, useReducedMotion } from 'framer-motion';
import type { FaqItem } from '@/data/faqs/types';

// Animated disclosure accordion (single open at a time) with a smooth
// height + fade expand, like a polished demo-page FAQ. Answers stay mounted
// in the DOM (collapsed to height 0) so they remain crawlable, and FaqJsonLd
// carries the structured data. Mount sites must hydrate this component
// (client:visible on .astro pages, or inside an already-hydrated island).

type FaqTheme = 'dark' | 'light';

const themeStyles: Record<
	FaqTheme,
	{
		divider: string;
		question: string;
		answer: string;
		answerLinks: string;
		icon: string;
		heading: string;
		sectionBorder: string;
	}
> = {
	dark: {
		divider: 'divide-white/10 border-white/10',
		question: 'text-white',
		answer: 'text-zinc-400',
		answerLinks: '[&_a]:text-white [&_a]:underline [&_a]:underline-offset-2 [&_a:hover]:text-zinc-300 [&_strong]:text-zinc-200 [&_ul]:list-disc [&_ul]:pl-5 [&_ol]:list-decimal [&_ol]:pl-5 [&_li]:mt-1',
		icon: 'text-zinc-500',
		heading: 'text-white',
		sectionBorder: 'border-white/10',
	},
	light: {
		divider: 'divide-ink/10 border-ink/10',
		question: 'text-ink',
		answer: 'text-ink-soft',
		answerLinks: '[&_a]:text-pine [&_a]:underline [&_a]:underline-offset-2 [&_a:hover]:text-ink [&_strong]:text-ink [&_ul]:list-disc [&_ul]:pl-5 [&_ol]:list-decimal [&_ol]:pl-5 [&_li]:mt-1',
		icon: 'text-ink-faint',
		heading: 'text-ink',
		sectionBorder: 'border-ink/10',
	},
};

interface FaqListProps {
	items: FaqItem[];
	theme?: FaqTheme;
}

export function FaqList({ items, theme = 'dark' }: FaqListProps) {
	const styles = themeStyles[theme];
	const reduceMotion = useReducedMotion();
	const [openIndex, setOpenIndex] = useState<number | null>(null);

	return (
		<div className={`divide-y border-b ${styles.divider}`}>
			{items.map((item, index) => {
				const open = openIndex === index;
				const answerId = `faq-answer-${index}`;
				return (
					<div key={item.question} className="py-5">
						<button
							type="button"
							onClick={() => setOpenIndex(open ? null : index)}
							aria-expanded={open}
							aria-controls={answerId}
							className={`flex w-full cursor-pointer items-center justify-between gap-4 text-left text-base font-medium ${styles.question}`}
						>
							{item.question}
							<svg
								viewBox="0 0 16 16"
								aria-hidden="true"
								className={`h-4 w-4 flex-shrink-0 transition-transform duration-300 ease-out ${open ? 'rotate-45' : ''} ${styles.icon}`}
							>
								<path
									d="M8 2v12M2 8h12"
									stroke="currentColor"
									strokeWidth="1.5"
									strokeLinecap="round"
								/>
							</svg>
						</button>
						<motion.div
							id={answerId}
							initial={false}
							animate={{ height: open ? 'auto' : 0, opacity: open ? 1 : 0 }}
							transition={{ duration: reduceMotion ? 0 : 0.32, ease: [0.4, 0, 0.2, 1] }}
							style={{ overflow: 'hidden' }}
						>
							{/* Answers are first-party static strings from src/data/faqs, so rendering them as HTML is safe. */}
							<div
								className={`pt-3 text-sm leading-relaxed ${styles.answer} ${styles.answerLinks}`}
								dangerouslySetInnerHTML={{ __html: item.answerHtml }}
							/>
						</motion.div>
					</div>
				);
			})}
		</div>
	);
}

interface FaqSectionProps {
	items: FaqItem[];
	title?: string;
	theme?: FaqTheme;
	id?: string;
	className?: string;
}

export function FaqSection({
	items,
	title = 'Frequently asked questions',
	theme = 'dark',
	id,
	className = '',
}: FaqSectionProps) {
	const styles = themeStyles[theme];

	return (
		<section id={id} className={`border-t px-6 py-24 ${styles.sectionBorder} ${className}`}>
			<div className="mx-auto max-w-3xl">
				<h2
					className={`mb-12 text-center text-3xl font-medium tracking-[-0.015em] md:text-4xl ${styles.heading}`}
				>
					{title}
				</h2>
				<FaqList items={items} theme={theme} />
			</div>
		</section>
	);
}
