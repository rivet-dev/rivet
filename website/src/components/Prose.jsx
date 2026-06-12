import clsx from "clsx";

// Docs and other dark shells keep the inverted palette (default). Porcelain
// surfaces pass surface="paper" for the light palette.
export function Prose({ as: Component = "div", className = "", surface = "ink", ...props }) {
	return (
		<Component
			className={clsx(className, "prose", surface === "ink" && "prose-invert")}
			{...props}
		/>
	);
}
