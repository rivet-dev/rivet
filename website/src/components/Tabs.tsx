import { cn } from "@rivet-gg/components";
import type { ReactNode } from "react";

interface TabProps {
	title: string;
	children: ReactNode;
}

export const Tab = ({ title, children }: TabProps) => {
	return (
		<div data-tab-item data-tab-title={title}>
			<button
				type="button"
				data-tab-trigger={title}
				className={cn(
					"relative inline-flex min-h-[2.75rem] items-center justify-center whitespace-nowrap",
					"rounded-none border-b-2 bg-transparent px-4 py-2.5 text-sm font-semibold",
					"ring-offset-background transition-none",
					"focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2",
					"disabled:pointer-events-none disabled:opacity-50",
					"border-b-transparent text-muted-foreground",
				)}
			>
				{title}
			</button>
			<div data-tab-content className="hidden mt-4">
				{children}
			</div>
		</div>
	);
};

interface TabsProps {
	children: ReactNode;
}

export const Tabs = ({ children }: TabsProps) => {
	return (
		<div data-tabs-container className="my-4">
			<div className="overflow-x-auto">
				<div
					data-tabs-list
					className="inline-flex text-muted-foreground border-b w-full"
				>
				</div>
			</div>
			<div data-tabs-content-container>
			</div>
			<div data-tabs-source className="hidden">
				{children}
			</div>
		</div>
	);
};
