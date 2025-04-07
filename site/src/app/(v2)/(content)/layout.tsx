import { Prose } from "@/components/Prose";
import { FancyHeader } from "@/components/v2/FancyHeader";
import { cn } from "@rivet-gg/components";
import type { CSSProperties } from "react";

export default function Layout({ children }: { children: React.ReactNode }) {
	return (
		<>
			<FancyHeader />
			<div
				className="md:mt-32 mt-20 pb-8"
				style={{ "--header-height": "5rem" } as CSSProperties}
			>
				<Prose
					as="article"
					className={cn(
						"order-3 mt-4 w-full flex-shrink-0 lg:order-2 max-w-prose mx-auto",
					)}
				>
					{children}
				</Prose>
			</div>
		</>
	);
}
