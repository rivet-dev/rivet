import { faArrowUpRightFromSquare, Icon } from "@rivet-gg/icons";
import type { ReactNode } from "react";
import { type UseFormReturn, useFormContext, useWatch } from "react-hook-form";
import z from "zod";
import { PlanSummary, planSummaryProps } from "@/app/billing/plan-card";
import { ByocContactTrigger } from "@/app/byoc/byoc-contact-trigger";
import {
	CloudOrganizationSelect,
	cn,
	createSchemaForm,
	Flex,
	FormControl,
	FormDescription,
	FormField,
	FormItem,
	FormLabel,
	FormMessage,
	Input,
} from "@/components";
import { defineStepper } from "@/components/ui/stepper";
import type { PlanId } from "@/content/billing";
import { BYOC_DOCS_URL, BYOC_PLAN, BYOC_TRIAL_DAYS } from "@/content/byoc";
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
		description: (values: Record<string, unknown>) =>
			values.plan === "byoc" ? BYOC_PLAN.description : "",
		previous: "Back",
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
	const isByoc = useWatch({ control, name: "plan" }) === "byoc";
	return (
		<FormField
			control={control}
			name="name"
			render={({ field }) => (
				<FormItem className={className}>
					<FormLabel className="col-span-1">Name</FormLabel>
					<FormControl className="row-start-2">
						<Input
							placeholder={
								isByoc
									? "Enter a cluster name..."
									: "Enter a project name..."
							}
							autoFocus
							autoComplete="off"
							{...field}
						/>
					</FormControl>
					{isByoc ? (
						<FormDescription>
							Lowercase letters, numbers, and hyphens.
						</FormDescription>
					) : null}
					<FormMessage className="col-span-1" />
				</FormItem>
			)}
		/>
	);
};

export const Details = ({ children }: { children: ReactNode }) => {
	const { control } = useFormContext<FormValues>();
	const isByoc = useWatch({ control, name: "plan" }) === "byoc";

	if (!isByoc) {
		return (
			<Flex gap="4" direction="col">
				{children}
			</Flex>
		);
	}

	return (
		<div className="grid gap-6 lg:grid-cols-[minmax(0,1fr)_20rem]">
			<Flex gap="4" direction="col">
				{children}
			</Flex>
			<ClusterNextSteps />
		</div>
	);
};

const CLUSTER_NEXT_STEPS = [
	{
		title: "Download setup kit and cluster config",
		detail: "From your cluster page, or copy the instructions for your coding agent.",
	},
	{
		title: "Provision your infrastructure",
		detail: "Run Terraform in your AWS or Google Cloud account.",
	},
	{
		title: "Request deployment",
		detail: "Rivet deploys the control plane and notifies you.",
	},
] as const;

const ClusterNextSteps = () => (
	<aside className="rounded-lg border border-border bg-secondary/30 p-4 text-sm">
		<p className="font-medium">Next steps</p>
		<ol className="mt-3 space-y-3">
			{CLUSTER_NEXT_STEPS.map((step, index) => (
				<li
					key={step.title}
					className="grid grid-cols-[1.25rem_minmax(0,1fr)] gap-x-2"
				>
					<span className="font-mono-console text-xs leading-5 text-muted-foreground">
						{index + 1}.
					</span>
					<span>
						<span className="block leading-5">{step.title}</span>
						<span className="block text-xs text-muted-foreground">
							{step.detail}
						</span>
					</span>
				</li>
			))}
		</ol>
		<p className="mt-4 border-t border-border pt-3 text-xs text-muted-foreground">
			{BYOC_TRIAL_DAYS}-day free trial. Every BYOC cluster includes
			enterprise support.{" "}
			<ByocContactTrigger>
				{(open) => (
					<button
						type="button"
						className="underline underline-offset-4 hover:text-foreground"
						onClick={open}
					>
						Book a call
					</button>
				)}
			</ByocContactTrigger>
		</p>
	</aside>
);

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
