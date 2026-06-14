import { Icon, faArrowRight, faRivet, faServer } from '@rivet-gg/icons';
import type { ReactNode } from 'react';
import { FaqList } from '@/components/faq/FaqSection';
import { formatTimestamp } from '@/lib/formatDate';
import { compareEntries, getCompareEntry } from '@/data/compare';
import type { CompareEntry } from '@/data/compare/types';
import { CatalogCard } from '@/components/marketing/editorial/CatalogCard';
import { Spirograph } from '@/components/marketing/art/Spirograph';
import {
	CAPTION_CLASS,
	HERO_H1_CLASS,
	SECTION_H2_CLASS,
	SUBTITLE_CLASS,
} from '@/components/marketing/typography';
import { ComparisonTable } from './ComparisonTable';

// This component is server-rendered by Astro with no client directive, so it
// must stay hook-free with no framer-motion. Comparison pages are SEO entry
// pages and ship zero island JavaScript.
interface ComparePageProps {
	slug: string;
}

// Warm oil-paint texture behind the closing band, mirroring the sitewide CTA
// colophon. The veil keeps text readable even if the image fails to load.
const FOOTER_PAINTING_SRC =
	'https://assets.rivet.dev/website/images/textures/footer-abstract.webp';

function SectionHeading({
	title,
	subtitle,
	center = false,
}: {
	title: string;
	subtitle?: string;
	center?: boolean;
}) {
	return (
		<div className={center ? 'mx-auto max-w-2xl text-center' : 'max-w-2xl'}>
			<h2 className={SECTION_H2_CLASS}>{title}</h2>
			{subtitle && <p className={SUBTITLE_CLASS}>{subtitle}</p>}
		</div>
	);
}

function HeroSection({ entry }: { entry: CompareEntry }) {
	return (
		<section className="px-6 pb-24 pt-32 md:pb-28 md:pt-44">
			<div className="mx-auto w-full max-w-7xl">
				<div className="max-w-3xl">
					<h1 className={HERO_H1_CLASS}>
						{entry.rivetProductName} vs <br />
						{entry.competitorName}
					</h1>
					<p className="mt-7 max-w-2xl text-lg leading-8 text-ink-soft">{entry.heroSubtitle}</p>
					<div className="mt-10 flex flex-col gap-3 sm:flex-row sm:items-center">
						<a
							href="/docs/actors/quickstart/backend"
							className="inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md bg-accent-deep px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-accent"
						>
							Get Started with {entry.rivetProductName}
							<Icon icon={faArrowRight} />
						</a>
						<a
							href="/talk-to-an-engineer"
							className="inline-flex items-center justify-center whitespace-nowrap rounded-md border border-ink/20 px-4 py-2 text-sm text-ink-soft transition-colors hover:border-ink/40 hover:text-ink"
						>
							Talk to an engineer
						</a>
					</div>
					<p className={`mt-8 ${CAPTION_CLASS}`}>
						Last updated {formatTimestamp(entry.lastUpdated)}
					</p>
				</div>
			</div>
		</section>
	);
}

function ChoiceList({
	heading,
	choices,
	headingTone = 'faint',
}: {
	heading: string;
	choices: CompareEntry['whenToChooseRivet'];
	headingTone?: 'pine' | 'faint';
}) {
	return (
		<div>
			<div
				className={`font-mono text-[11px] font-medium uppercase tracking-[0.16em] ${
					headingTone === 'pine' ? 'text-pine' : 'text-ink-faint'
				}`}
			>
				{heading}
			</div>
			<div className="mt-5 space-y-5">
				{choices.map((choice) => (
					<div key={choice.title}>
						<div className="text-[15px] font-medium text-ink">{choice.title}</div>
						<div className="mt-1 text-sm leading-relaxed text-ink-soft">{choice.description}</div>
					</div>
				))}
			</div>
		</div>
	);
}

// Catalog-card treatment for the two overview plates. The Rivet plate is
// highlighted with a pine frame, so this stays local markup rather than the
// shared CatalogCard, whose hairline border is fixed.
function OverviewPanel({
	icon,
	name,
	summary,
	children,
	highlight = false,
}: {
	icon: ReactNode;
	name: string;
	summary: string;
	children: ReactNode;
	highlight?: boolean;
}) {
	return (
		<div
			className={`flex flex-col border bg-white/55 p-8 ${
				highlight ? 'border-pine/60' : 'border-ink/10'
			}`}
		>
			<div className="flex items-center gap-3">
				<span className={highlight ? 'text-pine' : 'text-olive'}>{icon}</span>
				<h3 className="text-lg font-medium tracking-[-0.01em] text-ink">{name}</h3>
			</div>
			<p className="mt-4 text-sm leading-relaxed text-ink-soft">{summary}</p>
			<div className="my-7 h-px bg-ink/10" />
			{children}
		</div>
	);
}

function OverviewSection({ entry }: { entry: CompareEntry }) {
	return (
		<section className="border-t border-ink/10 px-6 py-16 md:py-32">
			<div className="mx-auto max-w-7xl">
				<SectionHeading
					title="Two approaches, side by side"
					subtitle="What each platform is, and the situations where it is the right choice."
				/>
				<div className="mt-14 grid grid-cols-1 gap-6 lg:grid-cols-2">
					<OverviewPanel
						icon={<Icon icon={faRivet} className="h-4 w-4" />}
						name={entry.rivetProductName}
						summary={entry.rivetSummary}
						highlight
					>
						<ChoiceList
							heading={`When to choose ${entry.rivetProductName}`}
							choices={entry.whenToChooseRivet}
							headingTone="pine"
						/>
						<div className="mt-8">
							<a
								href="/docs/actors/quickstart/backend"
								className="group inline-flex items-center gap-2 text-sm font-medium text-pine hover:text-ink"
							>
								Get started with {entry.rivetProductName}
								<Icon
									icon={faArrowRight}
									className="transition-transform group-hover:translate-x-0.5"
								/>
							</a>
						</div>
					</OverviewPanel>

					<OverviewPanel
						icon={<Icon icon={entry.competitorIcon ?? faServer} className="h-4 w-4" />}
						name={entry.competitorName}
						summary={entry.competitorSummary}
					>
						<ChoiceList
							heading={`When to choose ${entry.competitorName}`}
							choices={entry.whenToChooseCompetitor}
						/>
					</OverviewPanel>
				</div>
			</div>
		</section>
	);
}

function ComparisonSection({ entry }: { entry: CompareEntry }) {
	return (
		<section className="border-t border-ink/10 px-6 py-16 md:py-32">
			<div className="mx-auto max-w-7xl">
				<SectionHeading
					title="Feature comparison"
					subtitle="A detailed breakdown of capabilities across both platforms."
				/>
				<div className="mt-14">
					<ComparisonTable
						featureGroups={entry.featureGroups}
						competitorName={entry.competitorName}
						competitorIcon={entry.competitorIcon}
						rivetProductName={entry.rivetProductName}
					/>
				</div>
			</div>
		</section>
	);
}

function VerdictSection({ entry }: { entry: CompareEntry }) {
	return (
		<section className="border-t border-ink/10 px-6 py-16 md:py-32">
			<div className="mx-auto max-w-2xl text-center">
				<SectionHeading title="Which should you pick?" center />
				<div className="mt-8 space-y-5">
					{entry.verdict.map((paragraph, index) => (
						<p
							key={paragraph}
							className={
								index === 0
									? 'text-lg leading-8 text-ink'
									: 'text-base leading-relaxed text-ink-soft'
							}
						>
							{paragraph}
						</p>
					))}
				</div>
			</div>
		</section>
	);
}

function MigrationSection({ migration }: { migration: NonNullable<CompareEntry['migration']> }) {
	return (
		<section className="border-t border-ink/10 px-6 py-16 md:py-32">
			<div className="mx-auto max-w-2xl text-center">
				<SectionHeading title={migration.heading} center />
				<p className="mt-8 text-base leading-relaxed text-ink-soft">{migration.body}</p>
				<div className="mt-8">
					<a
						href="/talk-to-an-engineer"
						className="inline-flex items-center justify-center whitespace-nowrap rounded-md border border-ink/20 px-4 py-2 text-sm text-ink-soft transition-colors hover:border-ink/40 hover:text-ink"
					>
						Talk to an engineer
					</a>
				</div>
			</div>
		</section>
	);
}

function FaqSectionLight({ entry }: { entry: CompareEntry }) {
	return (
		<section className="border-t border-ink/10 px-6 py-16 md:py-32">
			<div className="mx-auto max-w-3xl">
				<SectionHeading title="Frequently asked questions" />
				<div className="mt-10">
					<FaqList items={entry.faq} theme="light" />
				</div>
			</div>
		</section>
	);
}

function OtherComparisonsSection({ entry }: { entry: CompareEntry }) {
	const others = compareEntries.filter((other) => other.slug !== entry.slug);
	if (others.length === 0) {
		return null;
	}

	return (
		<section className="border-t border-ink/10 px-6 py-16 md:py-32">
			<div className="mx-auto max-w-7xl">
				<SectionHeading title="Other comparisons" />
				<div className="mt-12 grid grid-cols-1 gap-6 md:grid-cols-2">
					{others.map((other) => (
						<CatalogCard
							key={other.slug}
							href={`/compare/${other.slug}/`}
							title={other.title}
							linkLabel="Read the comparison"
						>
							<p className="mt-2 text-sm leading-relaxed text-ink-soft">{other.description}</p>
						</CatalogCard>
					))}
				</div>
			</div>
		</section>
	);
}

function CTASection() {
	return (
		<section className="selection-paper bg-paper px-4 py-14 text-center text-cream md:px-6 md:py-24">
			<div className="relative mx-auto flex min-h-[26rem] max-w-screen-2xl items-center justify-center overflow-hidden px-6 py-20 md:min-h-[34rem] md:px-10 lg:aspect-[2563/1440] lg:min-h-0">
				<img
					aria-hidden="true"
					src={FOOTER_PAINTING_SRC}
					alt=""
					loading="lazy"
					decoding="async"
					className="absolute inset-0 h-full w-full object-contain object-center"
				/>
				<div
					aria-hidden="true"
					className="absolute inset-0"
					style={{
						background:
							'linear-gradient(180deg, rgba(20,19,16,0.66), rgba(20,19,16,0.5) 50%, rgba(20,19,16,0.72))',
					}}
				/>
				<div className="relative mx-auto max-w-3xl">
					<div className="mb-8 flex justify-center" aria-hidden="true">
						<Spirograph
							variant="moire"
							size={56}
							stroke="#93A286"
							strokeWidth={2.6}
							strokeOpacity={0.7}
							copies={12}
						/>
					</div>
					<h2 className="text-3xl font-medium tracking-[-0.015em] text-cream md:text-5xl">
						The primitive for stateful workloads.
					</h2>
					<p className="mt-6 text-base leading-relaxed text-cream/65">
						The next generation of software needs a new kind of backend. This is it.
					</p>
					<div className="mt-10 flex flex-col items-center justify-center gap-3 sm:flex-row">
						<a
							href="/docs"
							className="inline-flex items-center justify-center whitespace-nowrap rounded-md bg-cream px-4 py-2 text-sm font-medium text-ink transition-colors hover:bg-white"
						>
							Start Building
						</a>
						<a
							href="/talk-to-an-engineer"
							className="inline-flex items-center justify-center whitespace-nowrap rounded-md border border-cream/30 px-4 py-2 text-sm text-cream transition-colors hover:border-cream/60"
						>
							Talk to an Engineer
						</a>
					</div>
				</div>
			</div>
		</section>
	);
}

export function ComparePage({ slug }: ComparePageProps) {
	const entry = getCompareEntry(slug);
	if (!entry) {
		throw new Error(`Unknown compare entry: ${slug}`);
	}

	return (
		<div className="paper-grain min-h-screen font-sans text-ink-soft">
			<main>
				<HeroSection entry={entry} />
				<OverviewSection entry={entry} />
				<ComparisonSection entry={entry} />
				<VerdictSection entry={entry} />
				{entry.migration && <MigrationSection migration={entry.migration} />}
				<FaqSectionLight entry={entry} />
				<OtherComparisonsSection entry={entry} />
				<CTASection />
			</main>
		</div>
	);
}
