import type { ReactNode } from 'react';
import { EYEBROW_CLASS } from '../typography';

// Museum-catalog section label: an optional mono index, an em dash, and a
// letter-spaced uppercase label in pine. Hook-free so zero-JS pages can use it.
interface EyebrowProps {
	label: ReactNode;
	index?: string;
	rule?: boolean;
	className?: string;
}

export const Eyebrow = ({ label, index, rule = false, className }: EyebrowProps) => (
	<p
		className={`${EYEBROW_CLASS} ${rule ? 'border-t border-ink/10 pt-5' : ''} ${className ?? ''}`}
	>
		{index ? <span className="text-ink-faint">{index} — </span> : null}
		{label}
	</p>
);
