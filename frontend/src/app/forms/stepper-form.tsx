/** biome-ignore-all lint/complexity/noBannedTypes: its okay */
import { zodResolver } from "@hookform/resolvers/zod";
import type * as Stepperize from "@stepperize/react";
import { type ReactNode, useEffect, useRef, useState } from "react";
import {
	FormProvider,
	type UseFormProps,
	type UseFormReturn,
	useForm,
} from "react-hook-form";
import type * as z from "zod";
import { Button } from "@/components";
import type { defineStepper } from "@/components/ui/stepper";
import { HelpDropdown } from "../help-dropdown";

type Step = Stepperize.Step & {
	assist: boolean;
	schema: z.ZodSchema;
	next?: string;
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
		content: Record<Steps[number]["id"], () => ReactNode>;
	};

export type StepperFormValues<Steps extends Step[]> = z.TypeOf<
	Steps[number]["schema"]
>;

export type ExtractSteps<T> = T extends StepperFormProps<infer Steps>
	? Steps
	: never;

export function StepperForm<const Steps extends Step[]>(
	props: StepperFormProps<Steps>,
) {
	const Stepper = props.Stepper;
	return (
		<Stepper.Provider variant="vertical">
			<Content<Steps> {...props} />
		</Stepper.Provider>
	);
}

function Content<const Steps extends Step[]>({
	defaultValues,
	Stepper,
	useStepper,
	content,
	onSubmit,
	...formProps
}: StepperFormProps<Steps>) {
	const stepper = useStepper();
	const form = useForm<z.infer<JoinStepSchemas<Steps>>>({
		defaultValues,
		resolver: zodResolver(stepper.current.schema),
		...formProps,
	});

	const ref = useRef<z.infer<JoinStepSchemas<Steps>> | null>({});

	const handleSubmit = (values: z.infer<JoinStepSchemas<Steps>>) => {
		ref.current = { ...ref.current, ...values };
		if (stepper.isLast) {
			return onSubmit?.({ values: ref.current, form, stepper });
		}
		stepper.next();
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
					{stepper.all.map((step) => (
						<Stepper.Step
							key={step.id}
							className="min-w-0"
							of={step.id}
						>
							<Stepper.Title>{step.title}</Stepper.Title>

							{stepper.when(step.id, (step) => {
								return (
									<Stepper.Panel className="space-y-6">
										{stepper.switch(content)}
										<Stepper.Controls>
											{step.assist ? (
												<NeedHelpButton />
											) : null}
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
											<Button
												type="submit"
												disabled={
													!form.formState.isValid
												}
												isLoading={
													form.formState.isSubmitting
												}
											>
												{step.next}
											</Button>
										</Stepper.Controls>
									</Stepper.Panel>
								);
							})}
						</Stepper.Step>
					))}
				</form>
			</FormProvider>
		</Stepper.Navigation>
	);
}

function NeedHelpButton() {
	const [open, setOpen] = useState(false);

	useEffect(() => {
		const timeout = setTimeout(() => {
			setOpen(true);
		}, 10000);
		return () => clearTimeout(timeout);
	}, []);

	if (!open) return null;

	return (
		<HelpDropdown>
			<Button variant="ghost">Need help?</Button>
		</HelpDropdown>
	);
}
