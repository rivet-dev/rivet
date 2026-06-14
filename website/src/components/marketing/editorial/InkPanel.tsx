import type { ReactNode } from 'react';

// The dark editorial plate. Every code, terminal, screenshot, and data moment
// on a porcelain marketing page lives inside an InkPanel and nothing else
// does. Hook-free so zero-JS pages can use it.
//
// `textureSrc` swaps the flat ink surface for a warm oil-painting backdrop
// under a darkening veil. Reserve it for editorial moments (CTA colophon,
// 404); code and data plates stay flat ink for legibility. If the image is
// missing the veil simply sits on flat ink, so nothing blocks on asset
// uploads.
interface InkPanelProps {
	children: ReactNode;
	caption?: ReactNode;
	captionAside?: ReactNode;
	bleed?: boolean;
	textureSrc?: string;
	texturePosition?: string;
	className?: string;
}

export const InkPanel = ({
	children,
	caption,
	captionAside,
	bleed = false,
	textureSrc,
	texturePosition = 'center',
	className,
}: InkPanelProps) => (
	<div
		className={`selection-paper relative overflow-hidden bg-ink text-cream ${
			bleed ? '' : 'rounded-xl border border-ink/20'
		} ${className ?? ''}`}
	>
		{textureSrc ? (
			<>
				<div
					aria-hidden="true"
					className="absolute inset-0 bg-cover"
					style={{ backgroundImage: `url('${textureSrc}')`, backgroundPosition: texturePosition }}
				/>
				<div
					aria-hidden="true"
					className="absolute inset-0"
					style={{
						background:
							'linear-gradient(180deg, rgba(20,19,16,0.62), rgba(20,19,16,0.48) 50%, rgba(20,19,16,0.68))',
					}}
				/>
			</>
		) : null}
		<div className="relative">{children}</div>
		{caption ? (
			<div className="relative flex items-center justify-between gap-4 border-t border-cream/10 px-5 py-3 font-mono text-[11px] text-cream/45">
				<span>{caption}</span>
				{captionAside ? <span>{captionAside}</span> : null}
			</div>
		) : null}
	</div>
);

// One-line command strip, e.g. `$ npm install rivetkit`. The `$` renders in
// sage on ink, pine on paper; pass the command without it. The default ink
// variant is a deliberate dark moment; use the paper variant where several
// commands sit near each other and solid ink would dominate the composition.
interface InkChipProps {
	command: string;
	variant?: 'ink' | 'paper';
	className?: string;
}

export const InkChip = ({ command, variant = 'ink', className }: InkChipProps) => (
	<div
		className={`flex items-center gap-2.5 overflow-x-auto rounded-md font-mono text-[13px] ${
			variant === 'ink'
				? 'selection-paper border border-ink/20 bg-ink px-4 py-3 text-cream/85'
				: 'border border-ink/15 bg-white/55 px-3.5 py-2.5 text-ink-soft'
		} ${className ?? ''}`}
	>
		<span aria-hidden="true" className={`select-none ${variant === 'ink' ? 'text-sage' : 'text-pine'}`}>
			$
		</span>
		<span className="whitespace-nowrap">{command}</span>
	</div>
);
