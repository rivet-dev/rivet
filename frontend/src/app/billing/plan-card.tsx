import { faCheck, faPlus, Icon, type IconProp } from "@rivet-gg/icons";
import type { ReactNode } from "react";
import { Button, cn } from "@/components";

type PlanCardProps = {
	title: string;
	price: string;
	features: { icon: IconProp; label: ReactNode }[];
	usageBased?: boolean;
	custom?: boolean;
	current?: boolean;
	buttonProps?: React.ComponentProps<typeof Button>;
} & React.ComponentProps<"div">;

function PlanCard({
	title,
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
			<h3 className="text-lg font-medium mb-2">{title}</h3>
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
			<div className="text-sm text-primary-foreground border-t pt-2 flex-1">
				<p>Includes:</p>
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
					>
						Current Plan
					</Button>
				) : (
					<Button className="w-full mt-4" {...buttonProps}>
						{custom ? "Contact Us" : "Upgrade"}
					</Button>
				)
			) : null}
		</div>
	);
}

export const CommunityPlan = (props: Partial<PlanCardProps>) => {
	return (
		<PlanCard
			title="Free"
			price="$0"
			features={[
				{ icon: faCheck, label: "5GB Limit" },
				{ icon: faCheck, label: "5 Million Writes /mo" },
				{ icon: faCheck, label: "200 Million Reads /mo" },
				{ icon: faCheck, label: "Community Support" },
			]}
			{...props}
		/>
	);
};

export const ProPlan = (props: Partial<PlanCardProps>) => {
	return (
		<PlanCard
			title="Hobby"
			price="$20"
			usageBased
			features={[
				{
					icon: faPlus,
					label: "20 Billion Read /mo",
				},
				{
					icon: faPlus,
					label: "50 Million Read /mo",
				},
				{
					icon: faPlus,
					label: "5GB Storage",
				},
				{ icon: faCheck, label: "Unlimited Seats" },
				{ icon: faCheck, label: "Email Support" },
			]}
			{...props}
		/>
	);
};

export const TeamPlan = (props: Partial<PlanCardProps>) => {
	return (
		<PlanCard
			title="Team"
			price="$200"
			usageBased
			features={[
				{ icon: faPlus, label: "25 Billion Reads /mo" },
				{ icon: faPlus, label: "50 Million Writes /mo" },
				{ icon: faPlus, label: "5GB Storage" },
				{ icon: faCheck, label: "Unlimited Seats" },
				{ icon: faCheck, label: "MFA" },
				{ icon: faCheck, label: "Slack Support" },
			]}
			{...props}
		/>
	);
};

export const EnterprisePlan = (props: Partial<PlanCardProps>) => {
	return (
		<PlanCard
			title="Enterprise"
			price="Custom"
			custom
			features={[
				{ icon: faCheck, label: "Everything in Team" },
				{ icon: faCheck, label: "Priority Support" },
				{ icon: faCheck, label: "SLA" },
				{ icon: faCheck, label: "OIDC SSO provider" },
				{ icon: faCheck, label: "On-Prem Deployment" },
				{ icon: faCheck, label: "Audit logs" },
				{ icon: faCheck, label: "Custom Roles" },
				{ icon: faCheck, label: "Device Tracking" },
			]}
			{...props}
		/>
	);
};
