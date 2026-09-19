import { faArrowRight, faLock, Icon } from "@rivet-gg/icons";
import { type UseFormReturn, useFormContext } from "react-hook-form";
import z from "zod";
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
import { PLANS } from "@/content/billing";
import { BYOC_DOCS_URL } from "@/content/byoc";
import { features } from "@/lib/features";

const SELECTABLE_PLANS = PLANS.filter((plan) => plan.id !== "enterprise");
const PLAN_FEATURE_COUNT = 4;

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
						<div className="space-y-3">
							<div className="grid grid-cols-1 sm:grid-cols-3 gap-3">
								{SELECTABLE_PLANS.map((plan) => (
									<button
										key={plan.id}
										type="button"
										aria-pressed={field.value === plan.id}
										onClick={() => field.onChange(plan.id)}
										className={cn(
											"flex flex-col rounded-lg border p-4 text-left transition-colors",
											"hover:bg-secondary/20",
											field.value === plan.id
												? "border-primary bg-secondary/20"
												: "border-border",
										)}
									>
										<span className="font-medium">
											{plan.title}
										</span>
										<span className="mt-1">
											<span className="text-2xl font-bold">
												{plan.price}
											</span>
											<span className="text-muted-foreground text-sm ml-1">
												/mo
											</span>
										</span>
										<ul className="mt-3 space-y-1 text-xs text-muted-foreground">
											{plan.features
												.slice(0, PLAN_FEATURE_COUNT)
												.map((feature) => (
													<li key={feature.label}>
														<Icon
															icon={feature.icon}
														/>{" "}
														{feature.label}
													</li>
												))}
										</ul>
									</button>
								))}
							</div>
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

const ByocPlanCard = ({
	isSelected,
	onSelect,
}: {
	isSelected: boolean;
	onSelect: () => void;
}) => {
	return (
		<button
			type="button"
			aria-pressed={isSelected}
			onClick={onSelect}
			className={cn(
				"flex w-full flex-col rounded-lg border p-4 text-left transition-colors",
				"hover:bg-secondary/20",
				isSelected ? "border-primary bg-secondary/20" : "border-border",
			)}
		>
			<span className="font-medium">BYOC</span>
			<span className="mt-1 text-sm text-muted-foreground">
				Run a fully-managed Rivet cluster inside your own cloud account.
			</span>
			<span className="mt-3 rounded-md border bg-secondary/30 px-3 py-2 text-xs text-muted-foreground">
				<Icon icon={faLock} className="text-primary mr-1.5" />
				<ByocContactTrigger>
					{(open) => (
						<button
							type="button"
							className="text-primary underline"
							onClick={(event) => {
								event.stopPropagation();
								open();
							}}
						>
							Contact us
						</button>
					)}
				</ByocContactTrigger>{" "}
				to add CMEK and other controls for your PCI/HIPAA needs.
			</span>
			<a
				href={BYOC_DOCS_URL}
				target="_blank"
				rel="noreferrer"
				className="mt-3 text-sm underline"
				onClick={(event) => event.stopPropagation()}
			>
				Read docs <Icon icon={faArrowRight} />
			</a>
		</button>
	);
};

export const DefaultSubmit = () => {
	return <Submit type="submit">Create Project</Submit>;
};
