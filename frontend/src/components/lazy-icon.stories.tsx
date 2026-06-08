import type { Story } from "@ladle/react";
import { faQuestion } from "@rivet-gg/icons";
import { Suspense } from "react";
import "../../.ladle/ladle.css";
import { ActorIcon, LazyIcon } from "./lazy-icon";

function Frame({ children }: { children: React.ReactNode }) {
	return (
		<div className="bg-background min-h-screen p-12">
			<div className="max-w-2xl space-y-8">{children}</div>
		</div>
	);
}

function Cell({
	label,
	children,
}: {
	label: string;
	children: React.ReactNode;
}) {
	return (
		<div className="flex flex-col items-center gap-2">
			<div className="flex h-9 w-9 items-center justify-center rounded-md bg-foreground/[0.06] text-foreground/80 text-lg">
				{children}
			</div>
			<span className="text-xs text-muted-foreground">{label}</span>
		</div>
	);
}

// Covers every branch of ActorIcon. Emoji and absent values render
// synchronously; a valid name lazily loads the real glyph; an unknown name and
// a null value both fall back to the default actor icon (this is the state that
// looked identical to the loading pulse and hid the re-render bug).
export const ActorIconStates: Story = () => (
	<Frame>
		<div className="flex flex-wrap gap-6">
			<Cell label="emoji">
				<ActorIcon iconValue="🚀" />
			</Cell>
			<Cell label="named (robot)">
				<ActorIcon iconValue="robot" />
			</Cell>
			<Cell label="named (kebab)">
				<ActorIcon iconValue="arrow-right" />
			</Cell>
			<Cell label="unknown name">
				<ActorIcon iconValue="not-a-real-icon-xyz" />
			</Cell>
			<Cell label="null (fallback)">
				<ActorIcon iconValue={null} />
			</Cell>
			<Cell label="custom fallback">
				<ActorIcon iconValue={null} fallback={faQuestion} />
			</Cell>
		</div>
	</Frame>
);

// LazyIcon is the low-level primitive: it always resolves to a glyph (the real
// icon, or the provided fallback for an unknown name) and the caller owns the
// Suspense boundary.
export const LazyIconPrimitive: Story = () => (
	<Frame>
		<Suspense fallback={null}>
			<div className="flex flex-wrap gap-6 text-lg">
				<Cell label="gear">
					<LazyIcon name="gear" fallback={faQuestion} />
				</Cell>
				<Cell label="user-group">
					<LazyIcon name="user-group" fallback={faQuestion} />
				</Cell>
				<Cell label="unknown → fallback">
					<LazyIcon name="nope-nope" fallback={faQuestion} />
				</Cell>
			</div>
		</Suspense>
	</Frame>
);
