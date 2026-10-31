import { Fragment } from "react";
import { WithTooltip } from "@/components/ui/tooltip";

/** Terms that get a dotted underline and an instant hover explanation. */
const TERMS: { pattern: RegExp; tooltip: string }[] = [
	{
		pattern: /availability zones?/g,
		tooltip:
			"An availability zone is an isolated data center within a region. Running across 3 zones keeps the cluster up through a zone outage, for better uptime.",
	},
	{
		pattern: /Actor Nodes/g,
		tooltip:
			"The servers that actually run your actors' code. Each node can run hundreds of actors. Add more nodes to run more actors.",
	},
];

/**
 * Renders a label or highlight string, underlining known terms with a hover
 * tooltip that explains them. Returns a single span so it stays one item
 * inside flex rows.
 */
export function Highlight({ text }: { text: string }) {
	const matches = TERMS.flatMap((term) =>
		[...text.matchAll(term.pattern)].map((m) => ({
			start: m.index ?? 0,
			end: (m.index ?? 0) + m[0].length,
			word: m[0],
			tooltip: term.tooltip,
		})),
	).sort((a, b) => a.start - b.start);

	const parts: React.ReactNode[] = [];
	let last = 0;
	for (const m of matches) {
		if (m.start < last) {
			continue;
		}
		if (m.start > last) {
			parts.push(
				<Fragment key={`t-${last}`}>
					{text.slice(last, m.start)}
				</Fragment>,
			);
		}
		parts.push(
			<WithTooltip
				key={`m-${m.start}`}
				delayDuration={0}
				content={<span className="block max-w-xs">{m.tooltip}</span>}
				trigger={
					<span className="cursor-help underline decoration-dotted decoration-muted-foreground/60 underline-offset-4">
						{m.word}
					</span>
				}
			/>,
		);
		last = m.end;
	}
	if (last < text.length) {
		parts.push(<Fragment key={`t-${last}`}>{text.slice(last)}</Fragment>);
	}
	return <span>{parts}</span>;
}
