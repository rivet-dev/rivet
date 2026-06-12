import { Icon, faArrowRight, faRivet, faServer } from '@rivet-gg/icons';
import type { ReactNode } from 'react';
import { FaqList } from '@/components/faq/FaqSection';
import { formatTimestamp } from '@/lib/formatDate';
import { compareEntries, getCompareEntry } from '@/data/compare';
import type { CompareEntry } from '@/data/compare/types';
import { ComparisonTable } from './ComparisonTable';

// This component is server-rendered by Astro with no client directive, so it
// must stay hook-free with no framer-motion. Comparison pages are SEO entry
// pages and ship zero island JavaScript.
interface ComparePageProps {
	slug: string;
}

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
			<h2 className="text-3xl font-medium tracking-[-0.015em] text-white md:text-4xl">{title}</h2>
			{subtitle && <p className="mt-4 text-base leading-relaxed text-zinc-500">{subtitle}</p>}
		</div>
	);
}

function HeroSection({ entry }: { entry: CompareEntry }) {
	return (
		<section className="px-6 pb-24 pt-32 md:pb-28 md:pt-44">
			<div className="mx-auto w-full max-w-7xl">
				<div className="max-w-3xl">
					<h1 className="text-4xl font-medium leading-[1.06] tracking-[-0.015em] text-white md:text-[3.75rem]">
						{entry.rivetProductName} vs <br />
						{entry.competitorName}
					</h1>
					<p className="mt-7 max-w-2xl text-lg leading-8 text-zinc-400">{entry.heroSubtitle}</p>
					<div className="mt-10 flex flex-col gap-3 sm:flex-row sm:items-center">
						<a
							href="/docs/actors/quickstart/backend"
							className="selection-dark inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md bg-white px-4 py-2 text-sm font-medium text-black transition-colors hover:bg-zinc-200"
						>
							Get Started with {entry.rivetProductName}
							<Icon icon={faArrowRight} />
						</a>
						<a
							href="/talk-to-an-engineer"
							className="inline-flex items-center justify-center whitespace-nowrap rounded-md border border-white/10 px-4 py-2 text-sm text-zinc-300 transition-colors hover:border-white/20 hover:text-white"
						>
							Talk to an engineer
						</a>
					</div>
					<p className="mt-8 text-sm text-zinc-600">
						Last updated {formatTimestamp(entry.lastUpdated)}
					</p>
				</div>
			</div>
		</section>
	);
}

function ChoiceList({ heading, choices }: { heading: string; choices: CompareEntry['whenToChooseRivet'] }) {
	return (
		<div>
			<div className="text-[11px] font-medium uppercase tracking-wider text-zinc-600">{heading}</div>
			<div className="mt-5 space-y-5">
				{choices.map((choice) => (
					<div key={choice.title}>
						<div className="text-[15px] font-medium text-zinc-200">{choice.title}</div>
						<div className="mt-1 text-sm leading-relaxed text-zinc-500">{choice.description}</div>
					</div>
				))}
			</div>
		</div>
	);
}

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
			className={`flex flex-col rounded-xl border p-8 ${
				highlight ? 'border-white/15 bg-white/[0.03]' : 'border-white/10'
			}`}
		>
			<div className="flex items-center gap-3">
				<span className="text-zinc-500">{icon}</span>
				<h3 className="text-lg font-medium tracking-tight text-white">{name}</h3>
			</div>
			<p className="mt-4 text-sm leading-relaxed text-zinc-500">{summary}</p>
			<div className="my-7 h-px bg-white/10" />
			{children}
		</div>
	);
}

function OverviewSection({ entry }: { entry: CompareEntry }) {
	return (
		<section className="px-6 py-20 md:py-24">
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
						/>
						<div className="mt-8">
							<a
								href="/docs/actors/quickstart/backend"
								className="group inline-flex items-center gap-2 text-sm font-medium text-white"
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
		<section className="px-6 py-20 md:py-24">
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
		<section className="px-6 py-20 md:py-24">
			<div className="mx-auto max-w-2xl text-center">
				<SectionHeading title="Which should you pick?" center />
				<div className="mt-8 space-y-5">
					{entry.verdict.map((paragraph, index) => (
						<p
							key={paragraph}
							className={
								index === 0
									? 'text-lg leading-8 text-zinc-300'
									: 'text-base leading-relaxed text-zinc-500'
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
		<section className="px-6 py-20 md:py-24">
			<div className="mx-auto max-w-2xl text-center">
				<SectionHeading title={migration.heading} center />
				<p className="mt-8 text-base leading-relaxed text-zinc-500">{migration.body}</p>
				<div className="mt-8">
					<a
						href="/talk-to-an-engineer"
						className="inline-flex items-center justify-center whitespace-nowrap rounded-md border border-white/10 px-4 py-2 text-sm text-zinc-300 transition-colors hover:border-white/20 hover:text-white"
					>
						Talk to an engineer
					</a>
				</div>
			</div>
		</section>
	);
}

function FaqSectionDark({ entry }: { entry: CompareEntry }) {
	return (
		<section className="px-6 py-20 md:py-24">
			<div className="mx-auto max-w-3xl">
				<SectionHeading title="Frequently asked questions" />
				<div className="mt-10">
					<FaqList items={entry.faq} theme="dark" />
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
		<section className="px-6 py-20 md:py-24">
			<div className="mx-auto max-w-7xl">
				<SectionHeading title="Other comparisons" />
				<div className="mt-12 grid grid-cols-1 gap-6 md:grid-cols-2">
					{others.map((other) => (
						<a
							key={other.slug}
							href={`/compare/${other.slug}/`}
							className="group flex flex-col rounded-xl border border-white/10 p-7 transition-colors hover:border-white/25"
						>
							<h3 className="text-base font-medium tracking-tight text-white">{other.title}</h3>
							<p className="mt-2 text-sm leading-relaxed text-zinc-500">{other.description}</p>
							<span className="mt-5 inline-flex items-center gap-2 text-sm text-zinc-400 transition-colors group-hover:text-white">
								Read the comparison
								<Icon
									icon={faArrowRight}
									className="transition-transform group-hover:translate-x-0.5"
								/>
							</span>
						</a>
					))}
				</div>
			</div>
		</section>
	);
}

function CTASection() {
	return (
		<section className="border-t border-white/10 px-6 py-32 text-center md:py-44">
			<div className="mx-auto max-w-3xl">
				<h2 className="text-3xl font-medium tracking-[-0.015em] text-white md:text-5xl">
					The primitive for stateful workloads.
				</h2>
				<p className="mt-6 text-base leading-relaxed text-zinc-500">
					The next generation of software needs a new kind of backend. This is it.
				</p>
				<div className="mt-10 flex flex-col items-center justify-center gap-3 sm:flex-row">
					<a
						href="/docs"
						className="selection-dark inline-flex items-center justify-center whitespace-nowrap rounded-md bg-white px-4 py-2 text-sm font-medium text-black transition-colors hover:bg-zinc-200"
					>
						Start Building
					</a>
					<a
						href="/talk-to-an-engineer"
						className="inline-flex items-center justify-center whitespace-nowrap rounded-md border border-white/10 px-4 py-2 text-sm text-zinc-300 transition-colors hover:border-white/20 hover:text-white"
					>
						Talk to an Engineer
					</a>
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
		<div className="min-h-screen bg-black font-sans text-zinc-300 selection:bg-[#FF4500]/30 selection:text-orange-200">
			<main>
				<HeroSection entry={entry} />
				<OverviewSection entry={entry} />
				<ComparisonSection entry={entry} />
				<VerdictSection entry={entry} />
				{entry.migration && <MigrationSection migration={entry.migration} />}
				<FaqSectionDark entry={entry} />
				<OtherComparisonsSection entry={entry} />
				<CTASection />
			</main>
		</div>
	);
}
