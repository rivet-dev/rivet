// Export theatrical components
export {
	Separator,
	Narrative,
	Concept,
	Direction,
	TheatricalCode,
	ActEnd,
} from "./theatrical";

// Basic HTML element overrides with theatrical styling
export const a = (props: any) => <a {...props} />;

export const Image = ({ className, ...props }: any) => (
	<img {...props} className={`w-auto h-auto mx-auto ${className || ""}`} />
);

export const h2 = function H2(props: any) {
	return (
		<h2
			className="font-display text-[#e7e5e4] text-2xl md:text-3xl mb-6 mt-16 tracking-wide uppercase"
			{...props}
		/>
	);
};

export const h3 = function H3(props: any) {
	return (
		<h3
			className="font-display text-[#e7e5e4] text-xl md:text-2xl mb-4 mt-12"
			{...props}
		/>
	);
};

export const p = function P(props: any) {
	return <p className="font-serif text-lg md:text-xl text-[#d6d3d1] leading-relaxed mb-6" {...props} />;
};

export const ul = function UL(props: any) {
	return <ul className="font-serif text-lg md:text-xl text-[#d6d3d1] mb-6 space-y-2 list-none pl-6" {...props} />;
};

export const ol = function OL(props: any) {
	return <ol className="font-serif text-lg md:text-xl text-[#d6d3d1] mb-6 space-y-2" {...props} />;
};

export const li = function LI(props: any) {
	return <li className="font-serif text-lg md:text-xl text-[#d6d3d1] before:content-['◆'] before:text-[#d4b483] before:mr-3 before:opacity-60" {...props} />;
};

export const blockquote = function Blockquote(props: any) {
	return (
		<blockquote
			className="border-l-2 border-[#d4b483] pl-6 my-8 italic text-[#a8a29e] text-lg"
			{...props}
		/>
	);
};

export const table = function Table(props: any) {
	return (
		<div className="overflow-x-auto my-8">
			<table className="w-full border-collapse border border-[#44403c]" {...props} />
		</div>
	);
};

export const th = function TH(props: any) {
	return (
		<th
			className="border border-[#44403c] bg-[#292524]/40 px-4 py-2 text-left text-[#d4b483] font-display"
			{...props}
		/>
	);
};

export const td = function TD(props: any) {
	return (
		<td
			className="border border-[#44403c] px-4 py-2 text-[#d6d3d1] font-serif"
			{...props}
		/>
	);
};

export const strong = function Strong(props: any) {
	return <strong className="text-[#e7e5e4] font-semibold" {...props} />;
};

export const em = function Em(props: any) {
	return <em className="text-[#d4b483] italic" {...props} />;
};

export const code = function Code(props: any) {
	return (
		<code
			className="font-mono text-sm bg-[#12100e] text-[#d4b483] px-1.5 py-0.5 rounded border border-[#44403c]"
			{...props}
		/>
	);
};

export const pre = function Pre(props: any) {
	return (
		<pre
			className="font-mono text-sm bg-[#12100e] border-l-2 border-[#d4b483] p-6 overflow-x-auto text-[#a8a29e] shadow-inner rounded-r-md my-8"
			{...props}
		/>
	);
};

export const hr = function HR(props: any) {
	return (
		<div className="flex items-center justify-center my-8 opacity-40">
			<div className="h-px w-12 bg-[#d4b483]" />
			<div className="mx-2 text-[#d4b483]">♦</div>
			<div className="h-px w-12 bg-[#d4b483]" />
		</div>
	);
};
