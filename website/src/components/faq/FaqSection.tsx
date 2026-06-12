import type { FaqItem } from '@/data/faqs/types';

// These components must stay hook-free with no framer-motion so they can be
// server-rendered by Astro without a client directive. Disclosure behavior
// comes from native <details>/<summary>, which works with zero JavaScript.

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

	return (
		<div className={`divide-y border-b ${styles.divider}`}>
			{items.map((item) => (
				<details key={item.question} className="group py-5">
					<summary
						className={`flex cursor-pointer list-none items-center justify-between gap-4 text-base font-medium [&::-webkit-details-marker]:hidden ${styles.question}`}
					>
						{item.question}
						<svg
							viewBox="0 0 16 16"
							aria-hidden="true"
							className={`h-4 w-4 flex-shrink-0 transition-transform duration-200 group-open:rotate-45 ${styles.icon}`}
						>
							<path
								d="M8 2v12M2 8h12"
								stroke="currentColor"
								strokeWidth="1.5"
								strokeLinecap="round"
							/>
						</svg>
					</summary>
					{/* Answers are first-party static strings from src/data/faqs, so rendering them as HTML is safe. */}
					<div
						className={`mt-3 text-sm leading-relaxed ${styles.answer} ${styles.answerLinks}`}
						dangerouslySetInnerHTML={{ __html: item.answerHtml }}
					/>
				</details>
			))}
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
