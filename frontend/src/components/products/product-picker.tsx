import type { ReactNode } from "react";
import { cn } from "@/components/lib/utils";
import { Badge } from "@/components/ui/badge";
import {
	getOnboardingTargetCopy,
	type OnboardingTarget,
} from "@/content/agent-prompts";
import { features } from "@/lib/features";
import { publicUrl } from "@/lib/utils";

// Products are SDKs the user builds on and deploys themselves. Services are
// managed by Rivet: the user only points a client at them, so they onboard
// through a different path and are listed under their own heading.
export type ProductSection = "products" | "services";

type Product = {
	target: OnboardingTarget;
	section: ProductSection;
	label: string;
	description: string;
	markFileName: string;
	badge?: string;
	isAvailable: () => boolean;
};

const PRODUCTS: Product[] = [
	{
		target: "actor",
		section: "products",
		label: "Actors",
		description: "The primitive for realtime, stateful workloads",
		markFileName: "actors-mark.svg",
		isAvailable: () => true,
	},
	{
		target: "agent-os",
		section: "products",
		label: "agentOS",
		description: "Hand every agent a computer of its own",
		markFileName: "agentos-mark.svg",
		isAvailable: () => features.agentOs,
	},
	{
		target: "workflows",
		section: "products",
		label: "Workflows",
		description: "Write multi-step operations that survive restarts",
		markFileName: "workflows-mark.svg",
		isAvailable: () => true,
	},
	{
		target: "dynamic-apps",
		section: "products",
		label: "Dynamic Apps",
		description: "Deploy AI-generated apps for your users",
		markFileName: "dynamic-apps-mark.svg",
		badge: "Preview",
		isAvailable: () => true,
	},
	{
		target: "durable-streams",
		section: "services",
		label: "Durable Streams",
		description: "Real-time streams with durable, replayable history",
		markFileName: "durable-streams-mark.svg",
		isAvailable: () => true,
	},
];

const SECTIONS: { id: ProductSection; label: string }[] = [
	{ id: "products", label: "Products" },
	{ id: "services", label: "Services" },
];

export function getAvailableProducts() {
	return PRODUCTS.filter((p) => p.isAvailable());
}

export function getProductSections() {
	const available = getAvailableProducts();
	return SECTIONS.map((section) => ({
		...section,
		products: available.filter((p) => p.section === section.id),
	})).filter((section) => section.products.length > 0);
}

export function getProduct(target: OnboardingTarget) {
	const product = PRODUCTS.find((p) => p.target === target);
	if (!product) {
		throw new Error(`Unknown product: ${target}`);
	}
	return product;
}

export function getProductDocsUrl(target: OnboardingTarget) {
	return getOnboardingTargetCopy(target).quickstartUrl;
}

// Product marks ship as tiles (logo inside a colored rounded square with an
// inner ring). Service marks are the bare third-party logo, so give them a
// neutral tile here to match the products' visual weight.
export function ProductMark({
	fileName,
	section,
	size = "md",
}: {
	fileName: string;
	section: ProductSection;
	/** `md` is the picker card size; `sm` fits inline in menus and text. */
	size?: "sm" | "md";
}) {
	const isService = section === "services";
	const img = (
		<img
			src={publicUrl(`images/brand/${fileName}`)}
			alt=""
			aria-hidden="true"
			className={cn(
				size === "md" ? "size-8" : "size-4",
				isService && (size === "md" ? "size-5" : "size-3"),
				isService && "dark:invert",
			)}
			draggable={false}
		/>
	);
	if (!isService) {
		return img;
	}
	return (
		<span
			className={cn(
				"flex items-center justify-center bg-muted",
				size === "md" ? "size-8 rounded-xl" : "size-4 rounded",
			)}
		>
			{img}
		</span>
	);
}

export function ProductCard({
	icon,
	label,
	description,
	badge,
	onSelect,
}: {
	icon: ReactNode;
	label: string;
	description: string;
	badge?: string;
	onSelect: () => void;
}) {
	return (
		<button
			type="button"
			onClick={onSelect}
			className="flex items-start gap-3 rounded-lg border border-border px-4 py-3 text-left transition-colors cursor-pointer hover:border-primary hover:bg-primary/5 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
		>
			<span className="mt-0.5 shrink-0">{icon}</span>
			<div className="min-w-0">
				<div className="flex items-center gap-2">
					<p className="text-sm font-medium whitespace-nowrap">
						{label}
					</p>
					{badge ? (
						<Badge
							variant="outline"
							className="shrink-0 text-[10px] leading-none py-0.5 px-1.5 font-medium"
						>
							{badge}
						</Badge>
					) : null}
				</div>
				<p className="text-xs text-muted-foreground">{description}</p>
			</div>
		</button>
	);
}

export const PRODUCT_COMPOSABILITY_NOTE =
	"Rivet is composable. Start with one product and add the rest to the same project whenever you need them.";

export function ProductPicker({
	onSelect,
	ariaLabel = "Select a product",
}: {
	onSelect: (target: OnboardingTarget) => void;
	ariaLabel?: string;
}) {
	return (
		<div>
			<fieldset
				aria-label={ariaLabel}
				className="m-0 min-w-0 border-0 p-0 space-y-4"
			>
				{getProductSections().map((section) => (
					<fieldset
						key={section.id}
						className="m-0 min-w-0 border-0 p-0"
					>
						<legend className="mb-2 text-xs font-medium uppercase tracking-wide text-muted-foreground">
							{section.label}
						</legend>
						<div className="grid grid-cols-1 gap-2 sm:grid-cols-2">
							{section.products.map((product) => (
								<ProductCard
									key={product.target}
									icon={
										<ProductMark
											fileName={product.markFileName}
											section={product.section}
										/>
									}
									label={product.label}
									description={product.description}
									badge={product.badge}
									onSelect={() => onSelect(product.target)}
								/>
							))}
						</div>
					</fieldset>
				))}
			</fieldset>
			<p className="mt-2 text-xs text-muted-foreground">
				{PRODUCT_COMPOSABILITY_NOTE}
			</p>
		</div>
	);
}
