import type { ReactNode } from "react";
import { Button, cn } from "@/components";
import { getPlan, type PlanId, type PlanRow } from "@/content/billing";
import { PlanBadge } from "./billing-plan-badge";

type PlanSummaryProps = {
	/** Plan key as in `PLAN_LABELS`; renders as the card's colored badge. */
	plan: string;
	price: string;
	/** Omitted where the surrounding step already frames the choice. */
	description?: string;
	rows: readonly PlanRow[];
	usageBased?: boolean;
	custom?: boolean;
	/** Rendered opposite the badge on the heading row. */
	tag?: ReactNode;
	className?: string;
};

/**
 * Plan header and spec table, matching the pricing page on rivet.dev. The
 * "From" line is always reserved so prices align across a row of cards.
 */
export function PlanSummary({
	plan,
	price,
	description,
	rows,
	usageBased,
	custom,
	tag,
	className,
}: PlanSummaryProps) {
	return (
		<div className={cn("flex flex-col", className)}>
			<div className="mb-3 flex items-center justify-between gap-3">
				<PlanBadge plan={plan} className="text-sm" />
				{tag}
			</div>
			<div className="mb-4">
				<span
					className={cn(
						"mb-1 block text-sm font-medium text-muted-foreground",
						!usageBased && "invisible",
					)}
					aria-hidden={!usageBased}
				>
					From
				</span>
				<div className="flex items-baseline gap-1">
					<span className="text-3xl font-medium tracking-[-0.015em]">
						{price}
					</span>
					{custom ? null : (
						<span className="ml-1 text-xs text-muted-foreground">
							{usageBased ? "/mo + Usage" : "/mo"}
						</span>
					)}
				</div>
			</div>
			{/* Without a description the table's top border is the divider. */}
			{description ? (
				<>
					<div className="mb-4 h-px bg-border" />
					<p className="mb-4 min-h-10 text-sm leading-5 text-muted-foreground">
						{description}
					</p>
				</>
			) : null}
			<dl className="divide-y divide-border border-y text-xs">
				{rows.map((row) => (
					<div
						key={row.label}
						className="flex items-baseline justify-between gap-3 py-2"
					>
						<dt
							className={
								row.value
									? "text-muted-foreground"
									: "text-foreground"
							}
						>
							{row.label}
						</dt>
						{row.value ? (
							<dd className="whitespace-nowrap text-right font-medium text-foreground">
								{row.value}
							</dd>
						) : null}
					</div>
				))}
			</dl>
		</div>
	);
}

export const planSummaryProps = (id: PlanId) => {
	const plan = getPlan(id);
	return {
		plan: id,
		price: plan.price,
		description: plan.description,
		rows: plan.rows,
		usageBased: "usageBased" in plan ? plan.usageBased : undefined,
		custom: "custom" in plan ? plan.custom : undefined,
	};
};

type PlanCardProps = PlanSummaryProps & {
	current?: boolean;
	buttonProps?: React.ComponentProps<typeof Button>;
} & Omit<React.ComponentProps<"div">, "className">;

function PlanCard({
	plan,
	price,
	description,
	rows,
	usageBased,
	custom,
	tag,
	current,
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
			<PlanSummary
				className="flex-1"
				plan={plan}
				price={price}
				description={description}
				rows={rows}
				usageBased={usageBased}
				custom={custom}
				tag={tag}
			/>
			{!buttonProps?.hidden ? (
				current ? (
					<Button
						variant="secondary"
						className="w-full mt-6"
						{...buttonProps}
					/>
				) : (
					<Button className="w-full mt-6" {...buttonProps} />
				)
			) : null}
		</div>
	);
}

export const CommunityPlan = (props: Partial<PlanCardProps>) => {
	return <PlanCard {...planSummaryProps("free")} {...props} />;
};

export const ProPlan = (props: Partial<PlanCardProps>) => {
	return <PlanCard {...planSummaryProps("pro")} {...props} />;
};

export const TeamPlan = (props: Partial<PlanCardProps>) => {
	return <PlanCard {...planSummaryProps("team")} {...props} />;
};

export const EnterprisePlan = (props: Partial<PlanCardProps>) => {
	return (
		<PlanCard
			{...planSummaryProps("enterprise")}
			{...props}
			buttonProps={{
				...props.buttonProps,
				children: "Contact Us",
			}}
		/>
	);
};
