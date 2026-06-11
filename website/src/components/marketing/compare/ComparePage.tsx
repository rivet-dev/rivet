'use client';

import { Icon, faArrowRight, faRivet, faServer } from '@rivet-gg/icons';
import { motion } from 'framer-motion';
import { FaqList } from '@/components/faq/FaqSection';
import { compareEntries, getCompareEntry } from '@/data/compare';
import type { CompareEntry } from '@/data/compare/types';
import { ComparisonTable } from './ComparisonTable';

// The route passes only the slug because CompareEntry contains JSX cell text,
// which is not serializable across the island boundary. The island looks the
// entry up from the registry itself.
interface ComparePageProps {
	slug: string;
}

function formatLastUpdated(isoDate: string): string {
	// Force UTC so the server render and client hydration produce the same
	// string regardless of local timezone.
	return new Date(isoDate).toLocaleDateString('en-US', {
		year: 'numeric',
		month: 'long',
		day: 'numeric',
		timeZone: 'UTC',
	});
}

function HeroSection({ entry }: { entry: CompareEntry }) {
	return (
		<section className="relative flex min-h-[60vh] flex-col justify-center px-6 pt-32 pb-24">
			<div className="mx-auto w-full max-w-7xl">
				<div className="max-w-3xl">
					<motion.h1
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5 }}
						className="mb-6 text-4xl font-normal leading-[1.1] tracking-tight text-white md:text-6xl"
					>
						{entry.rivetProductName} vs <br />
						{entry.competitorName}
					</motion.h1>
					<motion.p
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-base leading-relaxed text-zinc-500 md:text-lg"
					>
						{entry.heroSubtitle}
					</motion.p>
					<motion.p
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5, delay: 0.15 }}
						className="mt-4 text-sm text-zinc-600"
					>
						Last updated {formatLastUpdated(entry.lastUpdated)}
					</motion.p>
					<motion.div
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5, delay: 0.2 }}
						className="mt-8 flex flex-col gap-3 sm:flex-row"
					>
						<a
							href="/docs/actors/quickstart/backend"
							className="selection-dark inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md bg-white px-4 py-2 text-sm font-medium text-black transition-colors hover:bg-zinc-200"
						>
							Get Started with {entry.rivetProductName}
							<Icon icon={faArrowRight} />
						</a>
					</motion.div>
				</div>
			</div>
		</section>
	);
}

function OverviewSection({ entry }: { entry: CompareEntry }) {
	return (
		<section className="border-t border-white/10 py-24">
			<div className="mx-auto max-w-7xl px-6">
				<div className="mb-12">
					<motion.h2
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5 }}
						className="mb-2 text-2xl font-normal tracking-tight text-white md:text-4xl"
					>
						Overview
					</motion.h2>
					<motion.p
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="max-w-xl text-base leading-relaxed text-zinc-500"
					>
						Compare the two approaches and decide which is right for your project.
					</motion.p>
				</div>

				<div className="grid grid-cols-1 gap-8 md:grid-cols-2">
					<motion.div
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5 }}
						className="flex flex-col border-t border-white/10 pt-6"
					>
						<div className="mb-3 text-zinc-500">
							<Icon icon={faRivet} className="h-4 w-4" />
						</div>
						<h3 className="mb-2 text-base font-normal text-white">{entry.rivetProductName}</h3>
						<p className="mb-6 text-sm leading-relaxed text-zinc-500">{entry.rivetSummary}</p>

						<h4 className="text-sm font-medium uppercase tracking-wider text-zinc-500 mb-4">
							When to choose {entry.rivetProductName}
						</h4>
						<div className="mb-6 space-y-4">
							{entry.whenToChooseRivet.map((choice) => (
								<div key={choice.title}>
									<div className="text-sm text-white">{choice.title}</div>
									<div className="text-sm text-zinc-500">{choice.description}</div>
								</div>
							))}
						</div>
						<div className="mt-auto">
							<a
								href="/docs/actors/quickstart/backend"
								className="selection-dark inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md bg-white px-4 py-2 text-sm font-medium text-black transition-colors hover:bg-zinc-200"
							>
								Get started with {entry.rivetProductName}
								<Icon icon={faArrowRight} />
							</a>
						</div>
					</motion.div>

					<motion.div
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="flex flex-col border-t border-white/10 pt-6"
					>
						<div className="mb-3 text-zinc-500">
							<Icon icon={entry.competitorIcon ?? faServer} className="h-4 w-4" />
						</div>
						<h3 className="mb-2 text-base font-normal text-white">{entry.competitorName}</h3>
						<p className="mb-6 text-sm leading-relaxed text-zinc-500">{entry.competitorSummary}</p>

						<h4 className="text-sm font-medium uppercase tracking-wider text-zinc-500 mb-4">
							When to choose {entry.competitorName}
						</h4>
						<div className="space-y-4">
							{entry.whenToChooseCompetitor.map((choice) => (
								<div key={choice.title}>
									<div className="text-sm text-white">{choice.title}</div>
									<div className="text-sm text-zinc-500">{choice.description}</div>
								</div>
							))}
						</div>
					</motion.div>
				</div>
			</div>
		</section>
	);
}

function ComparisonSection({ entry }: { entry: CompareEntry }) {
	return (
		<section className="border-t border-white/10 py-24">
			<div className="mx-auto max-w-7xl px-6">
				<div className="mb-12">
					<motion.h2
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5 }}
						className="mb-2 text-2xl font-normal tracking-tight text-white md:text-4xl"
					>
						Feature Comparison
					</motion.h2>
					<motion.p
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="max-w-xl text-base leading-relaxed text-zinc-500"
					>
						A detailed breakdown of capabilities across both platforms.
					</motion.p>
				</div>
				<motion.div
					initial={{ opacity: 0, y: 20 }}
					whileInView={{ opacity: 1, y: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5, delay: 0.2 }}
				>
					<ComparisonTable
						featureGroups={entry.featureGroups}
						competitorName={entry.competitorName}
						competitorIcon={entry.competitorIcon}
						rivetProductName={entry.rivetProductName}
					/>
				</motion.div>
			</div>
		</section>
	);
}

function VerdictSection({ entry }: { entry: CompareEntry }) {
	return (
		<section className="border-t border-white/10 py-24 text-center">
			<div className="mx-auto max-w-3xl px-6">
				<motion.h2
					initial={{ opacity: 0, y: 20 }}
					whileInView={{ opacity: 1, y: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5 }}
					className="mb-3 text-2xl font-normal tracking-tight text-white md:text-4xl"
				>
					Verdict
				</motion.h2>
				{entry.verdict.map((paragraph) => (
					<motion.p
						key={paragraph}
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="mb-4 text-base leading-relaxed text-zinc-500 last:mb-0"
					>
						{paragraph}
					</motion.p>
				))}
			</div>
		</section>
	);
}

function MigrationSection({ migration }: { migration: NonNullable<CompareEntry['migration']> }) {
	return (
		<section className="border-t border-white/10 py-24 text-center">
			<div className="mx-auto max-w-3xl px-6">
				<motion.h2
					initial={{ opacity: 0, y: 20 }}
					whileInView={{ opacity: 1, y: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5 }}
					className="mb-3 text-2xl font-normal tracking-tight text-white md:text-4xl"
				>
					{migration.heading}
				</motion.h2>
				<motion.p
					initial={{ opacity: 0, y: 20 }}
					whileInView={{ opacity: 1, y: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5, delay: 0.1 }}
					className="mb-6 text-base leading-relaxed text-zinc-500"
				>
					{migration.body}
				</motion.p>
				<motion.div
					initial={{ opacity: 0, y: 20 }}
					whileInView={{ opacity: 1, y: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5, delay: 0.2 }}
				>
					<a
						href="/talk-to-an-engineer"
						className="inline-flex items-center justify-center whitespace-nowrap rounded-md border border-white/10 px-4 py-2 text-sm text-zinc-300 transition-colors hover:border-white/20 hover:text-white"
					>
						Talk to an engineer
					</a>
				</motion.div>
			</div>
		</section>
	);
}

function FaqSectionDark({ entry }: { entry: CompareEntry }) {
	return (
		<section className="border-t border-white/10 py-24">
			<div className="mx-auto max-w-3xl px-6">
				<motion.h2
					initial={{ opacity: 0, y: 20 }}
					whileInView={{ opacity: 1, y: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5 }}
					className="mb-12 text-center text-2xl font-normal tracking-tight text-white md:text-4xl"
				>
					Frequently asked questions
				</motion.h2>
				<FaqList items={entry.faq} theme="dark" />
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
		<section className="border-t border-white/10 py-24">
			<div className="mx-auto max-w-7xl px-6">
				<motion.h2
					initial={{ opacity: 0, y: 20 }}
					whileInView={{ opacity: 1, y: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5 }}
					className="mb-8 text-2xl font-normal tracking-tight text-white md:text-4xl"
				>
					Other comparisons
				</motion.h2>
				<div className="grid grid-cols-1 gap-6 md:grid-cols-2">
					{others.map((other) => (
						<a
							key={other.slug}
							href={`/compare/${other.slug}/`}
							className="group flex flex-col rounded-lg border border-white/10 p-6 transition-colors hover:border-white/20 hover:bg-white/5"
						>
							<h3 className="mb-2 text-base font-normal text-white">{other.title}</h3>
							<p className="text-sm leading-relaxed text-zinc-500">{other.description}</p>
							<span className="mt-4 inline-flex items-center gap-2 text-sm text-zinc-300 transition-colors group-hover:text-white">
								Read the comparison
								<Icon icon={faArrowRight} />
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
		<section className="border-t border-white/10 px-6 py-48 text-center">
			<div className="mx-auto max-w-3xl">
				<motion.h2
					initial={{ opacity: 0, y: 20 }}
					whileInView={{ opacity: 1, y: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5 }}
					className="mb-6 text-2xl font-normal tracking-tight text-white md:text-4xl"
				>
					The primitive for stateful workloads.
				</motion.h2>
				<motion.p
					initial={{ opacity: 0, y: 20 }}
					whileInView={{ opacity: 1, y: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5, delay: 0.1 }}
					className="mb-8 text-base leading-relaxed text-zinc-500"
				>
					The next generation of software needs a new kind of backend. This is it.
				</motion.p>
				<motion.div
					initial={{ opacity: 0, y: 20 }}
					whileInView={{ opacity: 1, y: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5, delay: 0.2 }}
					className="flex flex-col items-center justify-center gap-3 sm:flex-row"
				>
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
				</motion.div>
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
