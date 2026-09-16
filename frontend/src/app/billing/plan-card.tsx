import { Icon, type IconProp } from "@rivet-gg/icons";
import type { ReactNode } from "react";
import { Button, cn } from "@/components";
import { getPlan, type PlanId } from "@/content/billing";
import { PlanBadge } from "./billing-plan-badge";

type PlanCardProps = {
	/** Plan key as in `PLAN_LABELS`; renders as the card's colored badge. */
	plan: string;
	price: string;
	features: { icon: IconProp; label: ReactNode }[];
	usageBased?: boolean;
	custom?: boolean;
	current?: boolean;
	buttonProps?: React.ComponentProps<typeof Button>;
} & React.ComponentProps<"div">;

function PlanCard({
	plan,
	price,
	features,
	usageBased,
	current,
	custom,
	className,
	buttonProps,
	...props
}: PlanCardProps) {
	return (
		<div
			className={cn(
				"border rounded-lg p-6 h-full flex flex-col hover:bg-secondary/20 transition-colors",
				current && "border-primary",
				className,
			)}
			{...props}
		>
			<h3 className="mb-3">
				<PlanBadge plan={plan} className="text-sm" />
			</h3>
			<div className="min-h-24">
				{usageBased ? (
					<p className="text-xs text-muted-foreground">From</p>
				) : null}
				<p className="">
					<span className="text-4xl font-bold">{price}</span>
					{custom ? null : (
						<span className="text-muted-foreground ml-1">/mo</span>
					)}
				</p>
				{usageBased ? (
					<p className="text-sm text-muted-foreground">+ Usage</p>
				) : null}
			</div>
			<div className="text-sm text-primary-foreground flex-1">
				<ul className="text-muted-foreground mt-2 space-y-1">
					{features?.map((feature, index) => (
						<li key={`${feature.label}-${index}`}>
							<Icon icon={feature.icon} /> {feature.label}
						</li>
					))}
				</ul>
			</div>
			{!buttonProps?.hidden ? (
				current ? (
					<Button
						variant="secondary"
						className="w-full mt-4"
						{...buttonProps}
					/>
				) : (
					<Button className="w-full mt-4" {...buttonProps} />
				)
			) : null}
		</div>
	);
}

const fromCatalog = (id: PlanId) => {
	const plan = getPlan(id);
	return {
		plan: id,
		price: plan.price,
		usageBased: "usageBased" in plan ? plan.usageBased : undefined,
		custom: "custom" in plan ? plan.custom : undefined,
		features: [...plan.features],
	};
};

export const CommunityPlan = (props: Partial<PlanCardProps>) => {
	return <PlanCard {...fromCatalog("free")} {...props} />;
};

export const ProPlan = (props: Partial<PlanCardProps>) => {
	return <PlanCard {...fromCatalog("pro")} {...props} />;
};

export const TeamPlan = (props: Partial<PlanCardProps>) => {
	return <PlanCard {...fromCatalog("team")} {...props} />;
};

export const EnterprisePlan = (props: Partial<PlanCardProps>) => {
	return (
		<PlanCard
			{...fromCatalog("enterprise")}
			{...props}
			buttonProps={{
				...props.buttonProps,
				children: "Contact Us",
			}}
		/>
	);
};
