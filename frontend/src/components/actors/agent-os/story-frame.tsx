import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ReactNode } from "react";
import "../../../../.ladle/ladle.css";
import { TooltipProvider } from "@/components";

// Shared Ladle harness for the agentOS inspector tab stories. The tab
// components are presentational (data via props), so stories render them
// directly from fixtures inside a fixed-height card.

const queryClient = new QueryClient({
	defaultOptions: { queries: { retry: false, staleTime: Infinity } },
});

export function AgentOsStoryFrame({ children }: { children: ReactNode }) {
	return (
		<QueryClientProvider client={queryClient}>
			<TooltipProvider>
				<div className="bg-background p-6">
					<div className="h-[640px] w-full max-w-5xl overflow-hidden rounded-md border bg-card">
						{children}
					</div>
				</div>
			</TooltipProvider>
		</QueryClientProvider>
	);
}
