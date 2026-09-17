import { faArrowUpRightFromSquare, Icon } from "@rivet-gg/icons";
import type { ReactNode } from "react";
import { type UseFormReturn, useFormContext } from "react-hook-form";
import z from "zod";
import { PlanSummary, planSummaryProps } from "@/app/billing/plan-card";
import { ByocContactTrigger } from "@/app/byoc/byoc-contact-trigger";
import {
	CloudOrganizationSelect,
	cn,
	createSchemaForm,
	FormControl,
	FormField,
	FormItem,
	FormLabel,
	FormMessage,
	Input,
} from "@/components";
import { defineStepper } from "@/components/ui/stepper";
import type { PlanId } from "@/content/billing";
import { BYOC_DOCS_URL, BYOC_PLAN } from "@/content/byoc";
import { features } from "@/lib/features";

const SELECTABLE_PLANS = ["free", "pro", "team"] satisfies PlanId[];

export const planSchema = z.object({
	plan: z.enum(["free", "pro", "team", "byoc"]),
});

export const detailsSchema = z.object({
	name: z
		.string()
		.refine((value) => value.trim() !== "" && value.trim() === value, {
			message: "Name cannot be empty or contain whitespaces",
		}),
	organization: z.string().nonempty("Organization is required"),
});

export const byocDetailsSchema = detailsSchema.extend({
	name: z
		.string()
		.min(1)
		.max(63)
		.regex(
			/^[a-z0-9]([a-z0-9-]*[a-z0-9])?$/,
			"Must contain only lowercase alphanumeric characters and hyphens, and must not start or end with a hyphen",
		),
});

export const formSchema = z.object({
	...planSchema.shape,
	...detailsSchema.shape,
});

export type FormValues = z.infer<typeof formSchema>;
export type PlanValue = z.infer<typeof planSchema>["plan"];
export type SubmitHandler = (
	values: FormValues,
	form: UseFormReturn<FormValues>,
) => Promise<void>;

export const isPaidPlan = (plan: PlanValue) =>
	plan === "pro" || plan === "team";

export const stepper = defineStepper(
	{
		id: "plan",
		title: "Select plan",
		description: "You can change plans later.",
		next: "Continue",
		schema: planSchema,
	},
	{
		id: "details",
		title: "Create project",
		titleFor: (values: Record<string, unknown>) =>
			values.plan === "byoc" ? "Create cluster" : "Create project",
		next: "Continue",
		schema: (values: Record<string, unknown>) =>
			values.plan === "byoc" ? byocDetailsSchema : detailsSchema,
	},
	{
		id: "payment",
		title: "Payment method",
		description:
			"Add a payment method to activate the plan for this project.",
		next: "Finish",
		showPrevious: false,
		isVisible: (values: Record<string, unknown>) =>
			isPaidPlan(values.plan as PlanValue),
		schema: z.object({}),
	},
);

const { Form, Submit, SetValue } = createSchemaForm(formSchema);
export { Form, Submit, SetValue };

export const Name = ({ className }: { className?: string }) => {
	const { control } = useFormContext<FormValues>();
	return (
		<FormField
			control={control}
			name="name"
			render={({ field }) => (
				<FormItem className={className}>
					<FormLabel className="col-span-1">Name</FormLabel>
					<FormControl className="row-start-2">
						<Input
							placeholder="Enter a project name..."
							autoFocus
							autoComplete="off"
							{...field}
						/>
					</FormControl>
					<FormMessage className="col-span-1" />
				</FormItem>
			)}
		/>
	);
};

export const Organization = ({ className }: { className?: string }) => {
	const { control } = useFormContext<FormValues>();
	return (
		<FormField
			control={control}
			name="organization"
			render={({ field }) => (
				<FormItem className={className}>
					<FormLabel className="col-span-1">Organization</FormLabel>
					<FormControl className="row-start-2">
						<CloudOrganizationSelect
							onValueChange={field.onChange}
							value={field.value}
						/>
					</FormControl>
					<FormMessage className="col-span-1" />
				</FormItem>
			)}
		/>
	);
};

export const Plan = () => {
	const { control } = useFormContext<FormValues>();
	return (
		<FormField
			control={control}
			name="plan"
			render={({ field }) => (
				<FormItem>
					<FormControl>
						<div
							className={cn(
								"grid grid-cols-1 gap-3 sm:grid-cols-2",
								features.byoc
									? "lg:grid-cols-4"
									: "lg:grid-cols-3",
							)}
						>
							{SELECTABLE_PLANS.map((plan) => (
								<PlanOption
									key={plan}
									plan={plan}
									isSelected={field.value === plan}
									onSelect={() => field.onChange(plan)}
								/>
							))}
							{features.byoc ? (
								<ByocPlanCard
									isSelected={field.value === "byoc"}
									onSelect={() => field.onChange("byoc")}
								/>
							) : null}
						</div>
					</FormControl>
					<FormMessage />
				</FormItem>
			)}
		/>
	);
};

const PlanOptionButton = ({
	isSelected,
	onSelect,
	className,
	children,
}: {
	isSelected: boolean;
	onSelect: () => void;
	className?: string;
	children: ReactNode;
}) => (
	<button
		type="button"
		aria-pressed={isSelected}
		onClick={onSelect}
		className={cn(
			"flex flex-col rounded-lg border p-4 text-left transition-colors hover:bg-secondary/40",
			isSelected ? "border-primary bg-secondary/40" : "border-border",
			className,
		)}
	>
		{children}
	</button>
);

const PlanOption = ({
	plan,
	isSelected,
	onSelect,
}: {
	plan: PlanId;
	isSelected: boolean;
	onSelect: () => void;
}) => (
	<PlanOptionButton isSelected={isSelected} onSelect={onSelect}>
		<PlanSummary {...planSummaryProps(plan)} description={undefined} />
	</PlanOptionButton>
);

const ByocPlanCard = ({
	isSelected,
	onSelect,
}: {
	isSelected: boolean;
	onSelect: () => void;
}) => (
	<PlanOptionButton isSelected={isSelected} onSelect={onSelect}>
		<PlanSummary
			className="flex-1"
			plan="byoc"
			custom
			price={BYOC_PLAN.price}
			rows={BYOC_PLAN.rows}
			tag={
				<span className="text-xs text-muted-foreground">
					Your cloud
				</span>
			}
		/>
		<span className="mt-3 flex items-center gap-4 text-xs text-muted-foreground">
			<a
				href={BYOC_DOCS_URL}
				target="_blank"
				rel="noreferrer"
				className="hover:text-foreground"
				onClick={(event) => event.stopPropagation()}
			>
				Read docs{" "}
				<Icon icon={faArrowUpRightFromSquare} className="ml-0.5" />
			</a>
			<ByocContactTrigger>
				{(open) => (
					<button
						type="button"
						className="hover:text-foreground"
						onClick={(event) => {
							event.stopPropagation();
							open();
						}}
					>
						Talk to us
					</button>
				)}
			</ByocContactTrigger>
		</span>
	</PlanOptionButton>
);

export const DefaultSubmit = () => {
	return <Submit type="submit">Create Project</Submit>;
};
