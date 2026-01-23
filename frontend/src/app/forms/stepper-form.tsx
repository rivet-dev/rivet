import { zodResolver } from "@hookform/resolvers/zod";
import type * as Stepperize from "@stepperize/react";
import { posthog } from "posthog-js";
import { type ReactNode, useEffect, useMemo, useRef, useState } from "react";
import {
	FormProvider,
	type UseFormProps,
	type UseFormReturn,
	useForm,
	useFormContext,
} from "react-hook-form";
import * as z from "zod";
import { Button } from "@/components";
import type { defineStepper } from "@/components/ui/stepper";
import { HelpDropdown } from "../help-dropdown";

type Step = Stepperize.Step & {
	assist?: boolean;
	schema: z.ZodSchema;
	next?: string;
	showNext?: boolean;
};

type StepperProps<Steps extends Step[]> = ReturnType<
	typeof defineStepper<Steps>
>;

export type JoinStepSchemas<T extends Step[]> = T extends [
	infer First,
	...infer Rest,
]
	? First extends Step
		? First["schema"] extends z.ZodSchema
			? Rest extends Step[]
				? z.ZodIntersection<
						First["schema"],
						JoinStepSchemas<Rest> extends z.ZodTypeAny
							? JoinStepSchemas<Rest>
							: z.ZodObject<{}>
					>
				: First["schema"]
			: z.ZodObject<{}>
		: z.ZodObject<{}>
	: z.ZodObject<{}>;

type StepperFormProps<Steps extends Step[]> = StepperProps<Steps> &
	UseFormProps<z.infer<JoinStepSchemas<Steps>>> & {
		onSubmit?: (opts: {
			values: z.infer<JoinStepSchemas<Steps>>;
			form: UseFormReturn<z.infer<JoinStepSchemas<Steps>>>;
			stepper: ReturnType<Stepperize.StepperReturn<Steps>["useStepper"]>;
		}) => Promise<void> | void;
		onPartialSubmit?: (opts: {
			values: z.infer<JoinStepSchemas<Steps>>;
			form: UseFormReturn<z.infer<JoinStepSchemas<Steps>>>;
			stepper: ReturnType<Stepperize.StepperReturn<Steps>["useStepper"]>;
		}) => Promise<void> | void;
		content: Record<Steps[number]["id"], () => ReactNode>;
		showAllSteps?: boolean;
		initialStep?: Steps[number]["id"];
		footer?: ReactNode;
		children?: ReactNode;
		formId?: string;
	};

export type StepperFormValues<Steps extends Step[]> = z.TypeOf<
	Steps[number]["schema"]
>;

export type ExtractSteps<T> =
	T extends StepperFormProps<infer Steps> ? Steps : never;

export function StepperForm<const Steps extends Step[]>({
	children,
	...props
}: StepperFormProps<Steps>) {
	const Stepper = props.Stepper;
	return (
		<Stepper.Provider variant="vertical" initialStep={props.initialStep}>
			<Content<Steps> {...props} />
			{children}
		</Stepper.Provider>
	);
}

function Content<const Steps extends Step[]>({
	defaultValues,
	Stepper,
	useStepper,
	content,
	showAllSteps,
	onSubmit,
	onPartialSubmit,
	initialStep,
	footer,
	formId,
	...formProps
}: StepperFormProps<Steps>) {
	const stepper = useStepper({ initialStep });

	const mergedSchema = useMemo(() => {
		const schemas = stepper.all.map((step) => step.schema);
		if (schemas.length === 0) return null;
		return schemas
			.slice(1)
			.reduce(
				(acc, schema) => z.intersection(acc, schema),
				schemas[0] as z.ZodTypeAny,
			);
	}, [stepper.all]);

	const resolverSchema =
		showAllSteps && mergedSchema ? mergedSchema : stepper.current.schema;

	const form = useForm<z.infer<JoinStepSchemas<Steps>>>({
		defaultValues,
		resolver: zodResolver(resolverSchema),
		...formProps,
	});

	const ref = useRef<z.infer<JoinStepSchemas<Steps>> | null>({});

	const handleSubmit = async (values: z.infer<JoinStepSchemas<Steps>>) => {
		ref.current = { ...ref.current, ...values };

		if (formId) {
			posthog.capture(formId, {
				step: stepper.current.id,
				values: ref.current,
			});
		}
		if (stepper.isLast) {
			return onSubmit?.({ values: ref.current, form, stepper });
		}
		await onPartialSubmit?.({ values: ref.current, form, stepper });
		stepper.next();
		form.reset(undefined, {
			keepErrors: false,
			keepValues: true,
		});
	};

	return (
		<Stepper.Navigation>
			<FormProvider {...form}>
				<form
					onSubmit={(event) => {
						event.stopPropagation();
						return form.handleSubmit(handleSubmit)(event);
					}}
				>
					{stepper.all.map((step, index, steps) => (
						<Stepper.Step
							key={step.id}
							className="min-w-0 w-full"
							of={step.id}
						>
							<Stepper.Title>{step.title}</Stepper.Title>
							{step.assist && stepper.current.id === step.id ? (
								<Stepper.Helper>
									<NeedHelpButton />
								</Stepper.Helper>
							) : null}

							{showAllSteps ? (
								<StepPanel<Steps>
									key={step.id}
									Stepper={Stepper}
									stepper={stepper}
									step={step}
									content={content}
									showNext={false}
									showPrevious={false}
									showControls={steps.length - 1 === index}
									footer={footer}
								/>
							) : (
								stepper.when(step.id, (step) => {
									return (
										<StepPanel<Steps>
											Stepper={Stepper}
											stepper={stepper}
											step={step}
											content={content}
											footer={footer}
											showNext={step.showNext ?? true}
											showPrevious={
												step.showPrevious ?? true
											}
										/>
									);
								})
							)}
						</Stepper.Step>
					))}
				</form>
			</FormProvider>
		</Stepper.Navigation>
	);
}

function StepPanel<const Steps extends Step[]>({
	Stepper,
	stepper,
	step,
	content,
	showNext = true,
	showPrevious = true,
	showControls = true,
	footer,
}: Pick<StepperFormProps<Steps>, "Stepper" | "content"> & {
	stepper: Stepperize.Stepper<Steps>;
	step: Steps[number];
	showControls?: boolean;
	showNext?: boolean;
	showPrevious?: boolean;
	footer?: ReactNode;
}) {
	const form = useFormContext();

	return (
		<Stepper.Panel className="space-y-6">
			{stepper.match(step.id, content)}
			{showControls ? (
				<Stepper.Controls>
					{footer}
					{showPrevious && !stepper.isFirst ? (
						<Button
							type="button"
							variant="outline"
							onClick={() => {
								form.reset(undefined, {
									keepErrors: false,
									keepValues: true,
								});
								stepper.prev();
							}}
							disabled={stepper.isFirst}
						>
							Previous
						</Button>
					) : null}
					{showNext ? (
						<Button
							type="submit"
							disabled={!form.formState.isValid}
							isLoading={form.formState.isSubmitting}
						>
							{step.next || (stepper.isLast ? "Finish" : "Next")}
						</Button>
					) : null}
				</Stepper.Controls>
			) : null}
		</Stepper.Panel>
	);
}

function NeedHelpButton() {
	return (
		<HelpDropdown>
			<Button variant="link" className="text-foreground p-0">
				Need help?
			</Button>
		</HelpDropdown>
	);
}
