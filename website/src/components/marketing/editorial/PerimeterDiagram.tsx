import { Ban } from 'lucide-react';

// Green schematic of the self-hosted deployment boundary, shared by the
// landing OnPremSection and the enterprise page. Hook-free.
export interface PerimeterDiagramNode {
	title: string;
	detail: string;
}

const DEFAULT_NODES: PerimeterDiagramNode[] = [
	{ title: 'Your backend', detail: 'Actors run in your Node.js or Bun process' },
	{ title: 'Rivet Engine', detail: 'Single binary for orchestration and routing' },
	{ title: 'Your storage', detail: 'File system, Postgres, or FoundationDB' },
];

interface PerimeterDiagramProps {
	nodes?: PerimeterDiagramNode[];
	label?: string;
	footer?: string;
	className?: string;
}

export const PerimeterDiagram = ({
	nodes = DEFAULT_NODES,
	label = 'Your perimeter',
	footer = 'No outbound connections. No telemetry.',
	className,
}: PerimeterDiagramProps) => (
	<div className={`relative rounded-xl p-6 md:p-8 ${className ?? ''}`}>
		<svg
			aria-hidden='true'
			className='pointer-events-none absolute inset-0 h-full w-full overflow-visible'
			viewBox='0 0 100 100'
			preserveAspectRatio='none'
		>
			<rect
				x='0.75'
				y='0.75'
				width='98.5'
				height='98.5'
				rx='3.5'
				fill='none'
				stroke='currentColor'
				strokeWidth='1'
				strokeDasharray='4 3.2'
				className='perimeter-dash-rect text-ink/30'
				vectorEffect='non-scaling-stroke'
			/>
		</svg>
		<span className='absolute -top-2.5 left-6 z-10 bg-paper px-2 font-mono text-[11px] uppercase tracking-[0.14em] text-pine'>
			{label}
		</span>
		<div className='relative z-10 flex flex-col items-stretch'>
			{nodes.map((node, idx) => (
				<div key={node.title} className='flex flex-col'>
					{idx > 0 && <div className='mx-auto h-5 w-px bg-pine/40' />}
					<div className='rounded-lg border border-ink/10 bg-paper-deep/50 px-4 py-3'>
						<div className='text-sm font-medium text-ink'>{node.title}</div>
						<div className='mt-0.5 text-xs text-ink-soft'>{node.detail}</div>
					</div>
				</div>
			))}
		</div>
		<div className='relative z-10 mt-6 flex items-center gap-2 border-t border-ink/10 pt-4 font-mono text-[11px] text-pine'>
			<Ban className='h-3.5 w-3.5 text-pine/60' />
			{footer}
		</div>
	</div>
);
