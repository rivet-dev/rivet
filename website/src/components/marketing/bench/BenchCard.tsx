'use client';

// Benchmark card primitives: the dark "ink" data card with a mono eyebrow, a
// count-up headline, a comparison ledger, an optional toggle, and floating info
// tooltips. Used by the Actors marketing page (Actors vs. traditional
// infrastructure).

import { useId, useMemo, useState, useRef, useEffect } from 'react';
import type { ReactNode } from 'react';
import { motion, useReducedMotion } from 'framer-motion';
import { ArrowDown, ArrowUp } from 'lucide-react';
import { InkPanel } from '../editorial/InkPanel';

export function BenchInfoTooltip({ children }: { children: ReactNode }) {
	// The wrapper is intentionally not positioned so the tooltip spans the ink
	// card itself (the nearest positioned ancestor) instead of clipping at the
	// panel's overflow-hidden edge.
	return (
		<span className='group/tip ml-1.5 inline-flex align-middle'>
			<svg
				className='h-3.5 w-3.5 cursor-help text-cream/35 transition-colors group-hover/tip:text-cream/70'
				viewBox='0 0 16 16'
				fill='currentColor'
			>
				<path d='M8 0a8 8 0 100 16A8 8 0 008 0zm1 12H7V7h2v5zm-1-6a1 1 0 110-2 1 1 0 010 2z' />
			</svg>
			<span className='pointer-events-none absolute inset-x-3 bottom-12 z-50 rounded-lg border border-cream/15 bg-ink p-3 text-left text-[11px] leading-relaxed text-cream/80 opacity-0 shadow-xl transition-opacity duration-200 group-hover/tip:pointer-events-auto group-hover/tip:opacity-100 [&_a]:text-cream [&_a]:underline [&_a]:underline-offset-2 [&_strong]:font-medium [&_strong]:text-cream'>
				{children}
			</span>
		</span>
	);
}

export function BenchToggle({ options, active, onChange }: { options: string[]; active: number; onChange: (idx: number) => void }) {
	const layoutId = useId();
	const columns = options.length === 3 ? 'grid-cols-3' : options.length === 4 ? 'grid-cols-2' : 'grid-cols-2';

	return (
		<div className={`grid w-full gap-1 rounded-lg border border-cream/10 bg-cream/[0.03] p-1 ${columns}`}>
			{options.map((label, i) => {
				const isActive = i === active;
				return (
					<motion.button
						key={label}
						type='button'
						onClick={() => onChange(i)}
						aria-pressed={isActive}
						whileTap={{ scale: 0.94 }}
						className={`relative flex h-7 min-w-0 items-center justify-center rounded-md px-1.5 text-center font-mono text-[10px] uppercase tracking-[0.12em] transition-colors ${
							isActive ? 'text-ink' : 'text-cream/45 hover:text-cream/75'
						}`}
					>
						{isActive && (
							<motion.span
								layoutId={`bench-toggle-${layoutId}`}
								className='absolute inset-0 rounded-md bg-cream'
								transition={{ type: 'spring', stiffness: 480, damping: 38 }}
							/>
						)}
						<span className='relative z-[1] truncate'>{label}</span>
					</motion.button>
				);
			})}
		</div>
	);
}

export interface BenchRowEntry {
	label: ReactNode;
	value: string;
	highlight?: boolean;
}

// Splits a stat string into a leading symbol prefix, the numeric portion, and a
// trailing unit suffix so the number can be counted up while the units stay put.
// Returns null when there is no number to animate (e.g. "Infinite").
function parseStatNumber(text: string) {
	const match = text.match(/^([^\d-]*)(-?[\d,]*\.?\d+)(.*)$/);
	if (!match) return null;
	const [, prefix, rawNumber, suffix] = match;
	const normalized = rawNumber.replace(/,/g, '');
	const decimals = normalized.includes('.') ? normalized.split('.')[1].length : 0;
	return {
		prefix,
		suffix,
		value: Number.parseFloat(normalized),
		decimals,
		grouped: rawNumber.includes(','),
	};
}

// Counts the numeric part of a stat from 0 up to its value. The first run is
// gated on `active` (the card scrolling into view) and only fires once; later
// value changes (toggling workload or tier) re-trigger the count from the
// previous value. Honors reduced-motion by rendering the final value outright.
export function CountUpStat({ text, active }: { text: string; active: boolean }) {
	const parsed = useMemo(() => parseStatNumber(text), [text]);
	const reducedMotion = useReducedMotion();
	const target = parsed?.value ?? 0;

	const [display, setDisplay] = useState(0);
	const startedRef = useRef(false);
	const fromRef = useRef(0);
	const rafRef = useRef(0);

	useEffect(() => {
		if (!parsed) return;
		if (reducedMotion) {
			setDisplay(target);
			fromRef.current = target;
			startedRef.current = true;
			return;
		}
		// Not yet scrolled into view: stay primed at zero for the first count-up.
		if (!active) {
			if (!startedRef.current) setDisplay(0);
			return;
		}
		const from = startedRef.current ? fromRef.current : 0;
		startedRef.current = true;
		const duration = 850;
		let start = 0;
		const step = (now: number) => {
			if (!start) start = now;
			const t = Math.min(1, (now - start) / duration);
			const eased = 1 - (1 - t) ** 3;
			setDisplay(from + (target - from) * eased);
			if (t < 1) {
				rafRef.current = requestAnimationFrame(step);
			} else {
				fromRef.current = target;
			}
		};
		rafRef.current = requestAnimationFrame(step);
		return () => cancelAnimationFrame(rafRef.current);
	}, [parsed, target, active, reducedMotion]);

	if (!parsed) return <>{text}</>;

	const formatted = parsed.grouped
		? display.toLocaleString(undefined, {
				minimumFractionDigits: parsed.decimals,
				maximumFractionDigits: parsed.decimals,
			})
		: display.toFixed(parsed.decimals);

	return (
		<span className='tabular-nums'>
			{parsed.prefix}
			{formatted}
			{parsed.suffix}
		</span>
	);
}

// Dark ink data card with a mono title, direction tag, headline stat,
// and label/value rows pinned to the card's foot.
export function BenchCard({
	title,
	statNote,
	verb,
	direction = 'lower is better',
	toggle,
	rows,
	note,
}: {
	title: string;
	statNote: string;
	verb?: string;
	direction?: 'lower is better' | 'higher is better';
	toggle?: ReactNode;
	rows: BenchRowEntry[];
	note?: string;
}) {
	// Trigger the count-up the first time the card scrolls into view, once.
	const [inView, setInView] = useState(false);
	const isLower = direction !== 'higher is better';

	return (
		<InkPanel className='h-full'>
			<motion.div
				className='flex h-full flex-col p-6 md:p-7'
				onViewportEnter={() => setInView(true)}
				viewport={{ once: true, margin: '-10% 0px' }}
			>
				{/* Eyebrow rail */}
				<div className='flex min-h-[2.5rem] items-start justify-between gap-3'>
					<span className='font-mono text-[11px] font-medium uppercase tracking-[0.18em] text-sage'>{title}</span>
					<span className='inline-flex shrink-0 items-center gap-1 whitespace-nowrap font-mono text-[10px] uppercase tracking-[0.12em] text-cream/40'>
						{isLower ? <ArrowDown className='h-3 w-3 flex-shrink-0' /> : <ArrowUp className='h-3 w-3 flex-shrink-0' />}
						{direction}
					</span>
				</div>

				{/* Verdict: the headline multiplier */}
				<div className='mt-5 flex items-baseline gap-2'>
					<span className='font-sans text-[2.75rem] font-medium leading-[1.0] tracking-[-0.02em] tabular-nums text-cream md:text-5xl'>
						<CountUpStat text={statNote} active={inView} />
					</span>
					{verb ? <span className='font-sans text-lg font-medium text-cream/45 md:text-xl'>{verb}</span> : null}
				</div>

				{/* Comparison ledger: ours vs theirs, same unit, right-aligned */}
				<div className='mb-6 mt-6 divide-y divide-cream/10 border-y border-cream/10'>
					{rows.map((row, i) => (
						<div key={i} className='flex items-baseline justify-between gap-4 py-2.5'>
							<span className={`inline-flex min-w-0 items-baseline font-mono text-[13px] ${row.highlight ? 'font-medium text-cream' : 'font-normal text-cream/45'}`}>
								{row.label}
							</span>
							<span className={`whitespace-nowrap font-mono text-[15px] tabular-nums ${row.highlight ? 'font-medium text-sage' : 'font-normal text-cream/45'}`}>
								{row.value}
							</span>
						</div>
					))}
				</div>

				{toggle}
				{note ? (
					<p className='mt-auto font-mono text-[10px] leading-relaxed text-cream/35'>{note}</p>
				) : null}
			</motion.div>
		</InkPanel>
	);
}
