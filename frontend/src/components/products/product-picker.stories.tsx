import type { Story } from "@ladle/react";
import "../../../.ladle/ladle.css";
import { PRODUCT_COMPOSABILITY_NOTE, ProductPicker } from "./product-picker";

// The picker is rendered at two very different widths: full-bleed inside the
// onboarding step, and constrained inside the "Add a component" dialog. The
// two-column grid has to survive both.
export const InOnboardingStep: Story = () => (
	<div className="bg-background text-foreground min-h-screen p-8">
		<div className="max-w-xl">
			<h2 className="text-xl font-semibold">Select a product</h2>
			<p className="mt-1.5 mb-4 text-sm text-muted-foreground">
				{PRODUCT_COMPOSABILITY_NOTE}
			</p>
			<ProductPicker onSelect={() => {}} />
		</div>
	</div>
);

export const InDialog: Story = () => (
	<div className="bg-background text-foreground min-h-screen p-8">
		<div className="max-w-md rounded-lg border border-border p-6">
			<h2 className="text-lg font-semibold">Add a component</h2>
			<p className="text-sm text-muted-foreground mb-4">
				Pick what you want to add to this project.
			</p>
			<ProductPicker ariaLabel="Add a component" onSelect={() => {}} />
		</div>
	</div>
);

export const Narrow: Story = () => (
	<div className="bg-background text-foreground min-h-screen p-8">
		<div className="max-w-[320px]">
			<ProductPicker onSelect={() => {}} />
		</div>
	</div>
);
