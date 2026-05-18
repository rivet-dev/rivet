import { cn } from "./lib/utils";

// Inline SVG with three concave 4-point stars sized and positioned to mimic
// the `faSparkles` icon. Each star is animated independently via its own
// `animation-delay`, so they fade in/out one-by-one (twinkle effect) without
// scaling or all-fading-together. Size and color are inherited from the
// surrounding text (icon font model).
//
// The `animate-twinkle` keyframes live in `frontend/src/index.css`.
export function TwinklingSparkles({ className }: { className?: string }) {
	return (
		<svg
			viewBox="0 0 16 16"
			fill="currentColor"
			aria-hidden="true"
			className={cn("inline-block", className)}
		>
			{/* Large star, bottom-right cluster */}
			<path
				d="M10.5 1 L11.625 4.375 L15 5.5 L11.625 6.625 L10.5 10 L9.375 6.625 L6 5.5 L9.375 4.375 Z"
				className="animate-twinkle"
				style={{ animationDelay: "0s" }}
			/>
			{/* Medium star, bottom-left */}
			<path
				d="M4 8 L4.75 10.25 L7 11 L4.75 11.75 L4 14 L3.25 11.75 L1 11 L3.25 10.25 Z"
				className="animate-twinkle"
				style={{ animationDelay: "0.6s" }}
			/>
			{/* Small accent star, top-right */}
			<path
				d="M13 11 L13.5 12.5 L15 13 L13.5 13.5 L13 15 L12.5 13.5 L11 13 L12.5 12.5 Z"
				className="animate-twinkle"
				style={{ animationDelay: "1.2s" }}
			/>
		</svg>
	);
}
