import { ReactNode } from "react";

/**
 * Theatrical separator with ornamental styling
 */
export function Separator() {
	return (
		<div className="flex items-center justify-center my-8 opacity-40">
			<div className="h-px w-12 bg-[#d4b483]" />
			<div className="mx-2 text-[#d4b483]">♦</div>
			<div className="h-px w-12 bg-[#d4b483]" />
		</div>
	);
}

/**
 * Narrative block with drop cap first letter
 */
export function Narrative({ children }: { children: ReactNode }) {
	return (
		<p className="not-prose narrative-drop-cap text-lg md:text-xl leading-relaxed text-[#d6d3d1] mb-8 font-serif">
			{children}
		</p>
	);
}

/**
 * Concept block with border and title
 */
export function Concept({
	title,
	children,
}: {
	title: string;
	children: ReactNode;
}) {
	return (
		<div className="my-10 pl-6 border-l-2 border-[#d4b483]/50">
			<h3 className="text-[#d4b483] text-xl mb-2 font-bold">{title}</h3>
			<div className="text-[#a8a29e] italic text-lg md:text-xl">{children}</div>
		</div>
	);
}

/**
 * Stage direction block (centered, italicized)
 */
export function Direction({ children }: { children: ReactNode }) {
	return (
		<div className="my-8 text-center">
			<p className="font-serif italic text-[#78716c] text-base inline-block border-y border-[#44403c] py-2 px-6">
				{children}
			</p>
		</div>
	);
}

/**
 * Theatrical code block with vintage styling
 */
export function TheatricalCode({
	label,
	children,
}: {
	label?: string;
	children: ReactNode;
}) {
	return (
		<div className="my-8 font-mono text-sm relative group">
			{label && (
				<div className="absolute -top-3 left-4 bg-[#1c1917] px-2 text-[#d4b483] text-xs tracking-widest uppercase border border-[#d4b483]/30 z-10">
					{label}
				</div>
			)}
			<pre className="bg-[#12100e] border-l-2 border-[#d4b483] p-6 pt-8 overflow-x-auto text-[#a8a29e] shadow-inner rounded-r-md">
				<code>{children}</code>
			</pre>
		</div>
	);
}

/**
 * Act end marker
 */
export function ActEnd({ scene }: { scene: string }) {
	return (
		<div className="text-center my-12">
			<p className="font-mono text-xs text-[#57534e] uppercase tracking-widest">
				End of {scene}
			</p>
		</div>
	);
}
