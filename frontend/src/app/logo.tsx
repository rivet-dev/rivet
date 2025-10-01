import { ensureTrailingSlash } from "@/lib/utils";

export function Logo({ className }: { className?: string }) {
	return (
		<img
			src={`${ensureTrailingSlash(import.meta.env.BASE_URL || "")}logo.svg`}
			alt="Rivet.gg"
			className={className}
		/>
	);
}
