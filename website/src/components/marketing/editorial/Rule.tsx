import type { ReactNode } from 'react';
import { EYEBROW_CLASS } from '../typography';

// Hairline section divider. Sections separate with rules rather than
// background alternation; background shifts are reserved for ink panels and
// paper-deep bands. Hook-free.
export const Rule = ({ className }: { className?: string }) => (
	<hr className={`h-px border-0 bg-ink/10 ${className ?? ''}`} />
);

// Centered catalog label between two hairlines.
export const RuleLabeled = ({ label, className }: { label: ReactNode; className?: string }) => (
	<div className={`flex items-center gap-5 ${className ?? ''}`} role="separator">
		<span aria-hidden="true" className="h-px flex-1 bg-ink/10" />
		<span className={EYEBROW_CLASS}>{label}</span>
		<span aria-hidden="true" className="h-px flex-1 bg-ink/10" />
	</div>
);
