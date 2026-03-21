import { zodResolver } from "@hookform/resolvers/zod";
import { faArrowLeft, faArrowRight, Icon } from "@rivet-gg/icons";
import type * as Stepperize from "@stepperize/react";
import { AnimatePresence, motion } from "framer-motion";
import { posthog } from "posthog-js";
import {
	createContext,
	type MutableRefObject,
	type ReactNode,
	useContext,
	useRef,
} from "react";
import {
	FormProvider,
	type UseFormProps,
	type UseFormReturn,
	useForm,
	useFormContext,
} from "react-hook-form";
import type * as z from "zod";
import { Button } from "@/components";
import type { defineStepper } from "@/components/ui/stepper";
import { HelpDropdown } from "../help-dropdown";

type Step = Stepperize.Step & {
	assist?: boolean;
	schema: z.ZodSchema | ((values: Record<string, unknown>) => z.ZodSchema);
	next?: string;
	showNext?: boolean;
	showPrevious?: boolean;
	group?: string;
	isVisible?: (values: Record<string, unknown>) => boolean;
};

type StepVisibilityContextType = {
	isStepVisible: (stepId: string) => boolean;
	visibleStepIndex: (stepId: string) => number;
	visibleStepCount: number;
};

export const StepVisibilityContext = createContext<StepVisibilityContextType>({
	isStepVisible: () => true,
	visibleStepIndex: () => 0,
	visibleStepCount: 1,
});

type StepperProps<Steps extends Step[]> = ReturnType<
	typeof defineStepper<Steps>
>;

type StepperFormContextType = {
	submitForm: (() => void) | null;
};

const StepperFormContext = createContext<StepperFormContextType>({
	submitForm: null,
});

export const useStepperFormSubmit = () => {
	const context = useContext(StepperFormContext);
	return context.submitForm;
};

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
							: // biome-ignore lint/complexity/noBannedTypes: required for zod generic
								z.ZodObject<{}>
					>
				: First["schema"]
			: First["schema"] extends (
						values: Record<string, unknown>,
					) => z.ZodSchema
				? ReturnType<First["schema"]>
				: // biome-ignore lint/complexity/noBannedTypes: required for zod generic
					z.ZodObject<{}>
		: // biome-ignore lint/complexity/noBannedTypes: required for zod generic
			z.ZodObject<{}>
	: // biome-ignore lint/complexity/noBannedTypes: required for zod generic
		z.ZodObject<{}>;

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
		singlePage?: boolean;
		initialStep?: Steps[number]["id"];
		footer?: ReactNode;
		children?: ReactNode;
		formId?: string;
		className?: string;
	};

export type StepperFormValues<Steps extends Step[]> = z.TypeOf<
	Steps[number]["schema"] extends z.ZodSchema
		? Steps[number]["schema"]
		: Steps[number]["schema"] extends (
					values: Record<string, unknown>,
				) => z.ZodSchema
			? ReturnType<Steps[number]["schema"]>
			: never
>;

export type ExtractSteps<T> =
	T extends StepperFormProps<infer Steps> ? Steps : never;

export function StepperForm<const Steps extends Step[]>({
	children,
	className,
	...props
}: StepperFormProps<Steps>) {
	const Stepper = props.Stepper;
	return (
		<Stepper.Provider
			className={className}
			variant={props.singlePage ? "circle" : "vertical"}
			initialStep={props.initialStep}
		>
			<Content<Steps> {...props} extraChildren={children} />
		</Stepper.Provider>
	);
}

function useStepperDirection(stepper: {
	all: { id: string }[];
	current: { id: string };
}) {
	const currentStepIndex = stepper.all.findIndex(
		(s) => s.id === stepper.current.id,
	);
	const prevStepIndexRef = useRef(currentStepIndex);
	const directionRef = useRef(0);

	if (currentStepIndex !== prevStepIndexRef.current) {
		directionRef.current =
			currentStepIndex > prevStepIndexRef.current ? 1 : -1;
		prevStepIndexRef.current = currentStepIndex;
	}

	return directionRef.current;
}

function getNextVisibleStepId(
	steps: Step[],
	currentId: string,
	values: Record<string, unknown>,
): string | null {
	const currentIndex = steps.findIndex((s) => s.id === currentId);
	for (let i = currentIndex + 1; i < steps.length; i++) {
		const step = steps[i];
		if (!step.isVisible || step.isVisible(values)) return step.id;
	}
	return null;
}

function getPrevVisibleStepId(
	steps: Step[],
	currentId: string,
	values: Record<string, unknown>,
): string | null {
	const currentIndex = steps.findIndex((s) => s.id === currentId);
	for (let i = currentIndex - 1; i >= 0; i--) {
		const step = steps[i];
		if (!step.isVisible || step.isVisible(values)) return step.id;
	}
	return null;
}

function Content<const Steps extends Step[]>({
	defaultValues,
	Stepper,
	useStepper,
	steps: allSteps,
	content,
	showAllSteps,
	singlePage,
	onSubmit,
	onPartialSubmit,
	initialStep,
	footer,
	formId,
	extraChildren,
	...formProps
}: StepperFormProps<Steps> & { extraChildren?: ReactNode }) {
	const stepper = useStepper({ initialStep });

	const resolveSchema = (step: Step, values: Record<string, unknown>) => {
		if (typeof step.schema === "function") return step.schema(values);
		return step.schema;
	};

	const form = useForm<z.infer<JoinStepSchemas<Steps>>>({
		defaultValues,
		mode: "onChange",
		reValidateMode: "onChange",
		shouldFocusError: false,
		...formProps,
		...(stepper.current.schema
			? {
					resolver: (values, context, options) => {
						const accumulated = {
							...(ref.current ?? {}),
							...values,
						} as Record<string, unknown>;
						const schema = resolveSchema(
							stepper.current as unknown as Step,
							accumulated,
						);
						// @ts-expect-error - we know this is correct based on the definition of Step
						return zodResolver(schema)(values, context, options);
					},
				}
			: {}),
	});

	const ref = useRef<z.infer<JoinStepSchemas<Steps>> | null>({});
	const direction = useStepperDirection(stepper);
	const formRef = useRef<HTMLFormElement>(null);

	const getValues = () => {
		const allLive = form.getValues() as Record<string, unknown>;
		const dirtyFields = form.formState.dirtyFields as Record<
			string,
			unknown
		>;
		const live = Object.fromEntries(
			Object.entries(allLive).filter(([k]) => k in dirtyFields),
		);
		return { ...ref.current, ...live } as Record<string, unknown>;
	};

	const isLastVisible = (currentId: string) =>
		getNextVisibleStepId(allSteps as Step[], currentId, getValues()) ===
		null;

	const handleSubmit = async (values: z.infer<JoinStepSchemas<Steps>>) => {
		ref.current = { ...ref.current, ...values };

		if (formId) {
			posthog.capture(formId, {
				step: stepper.current.id,
				group: stepper.current.group,
				values: ref.current,
			});
		}
		if (isLastVisible(stepper.current.id)) {
			return onSubmit?.({ values: ref.current, form, stepper });
		}
		await onPartialSubmit?.({ values: ref.current, form, stepper });
		const nextId = getNextVisibleStepId(
			allSteps as Step[],
			stepper.current.id,
			getValues(),
		);
		if (nextId) stepper.goTo(nextId as Parameters<typeof stepper.goTo>[0]);
		form.reset(undefined, {
			keepErrors: false,
			keepValues: true,
			keepDirty: false,
		});
		form.clearErrors();
	};

	const visibleSteps = (allSteps as Step[]).filter(
		(s) => !s.isVisible || s.isVisible(getValues()),
	);
	const visibilityContext: StepVisibilityContextType = {
		isStepVisible: (stepId) => {
			const step = (allSteps as Step[]).find((s) => s.id === stepId);
			return !step?.isVisible || step.isVisible(getValues());
		},
		visibleStepIndex: (stepId) =>
			visibleSteps.findIndex((s) => s.id === stepId),
		visibleStepCount: visibleSteps.length,
	};

	if (singlePage) {
		const step = stepper.current;
		const isLast = isLastVisible(step.id);
		const hasPrev =
			getPrevVisibleStepId(allSteps as Step[], step.id, getValues()) !==
			null;
		return (
			<StepVisibilityContext.Provider value={visibilityContext}>
				<StepperFormContext.Provider
					value={{
						submitForm: () => formRef.current?.requestSubmit(),
					}}
				>
					<FormProvider {...form}>
						<form
							ref={formRef}
							onSubmit={(event) => {
								event.stopPropagation();
								return form.handleSubmit(handleSubmit)(event);
							}}
							className="space-y-6"
						>
							<AnimatePresence mode="wait" custom={direction}>
								<motion.div
									key={step.id}
									custom={direction}
									initial={{ opacity: 0, x: direction * 30 }}
									animate={{ opacity: 1, x: 0 }}
									exit={{ opacity: 0, x: direction * -30 }}
									transition={{
										duration: 0.25,
										ease: [0.4, 0, 0.2, 1],
									}}
								>
									<div className="flex items-center justify-between">
										<h2 className="text-xl font-semibold">
											{step.title}
										</h2>
										{step.assist ? (
											<NeedHelpButton />
										) : null}
									</div>
									<div className="mt-6">
										<StepPanel<Steps>
											Stepper={Stepper}
											stepper={stepper}
											allSteps={allSteps as Step[]}
											valuesRef={
												ref as MutableRefObject<Record<
													string,
													unknown
												> | null>
											}
											step={step}
											content={content}
											footer={footer}
											showNext={step.showNext ?? true}
											showPrevious={
												(step.showPrevious ?? true) &&
												hasPrev
											}
											isLastVisible={isLast}
										/>
									</div>
									{extraChildren}
								</motion.div>
							</AnimatePresence>
						</form>
					</FormProvider>
				</StepperFormContext.Provider>
			</StepVisibilityContext.Provider>
		);
	}

	return (
		<StepVisibilityContext.Provider value={visibilityContext}>
			<Stepper.Navigation>
				<StepperFormContext.Provider
					value={{
						submitForm: () => formRef.current?.requestSubmit(),
					}}
				>
					<FormProvider {...form}>
						<form
							ref={formRef}
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
									{step.assist &&
									stepper.current.id === step.id ? (
										<Stepper.Helper>
											<NeedHelpButton />
										</Stepper.Helper>
									) : null}

									{showAllSteps ? (
										<StepPanel<Steps>
											key={step.id}
											Stepper={Stepper}
											stepper={stepper}
											allSteps={allSteps as Step[]}
											valuesRef={
												ref as MutableRefObject<Record<
													string,
													unknown
												> | null>
											}
											step={step}
											content={content}
											showNext={false}
											showPrevious={false}
											showControls={
												steps.length - 1 === index
											}
											footer={footer}
											isLastVisible={isLastVisible(
												step.id,
											)}
										/>
									) : (
										stepper.when(step.id, (step) => {
											return (
												<StepPanel<Steps>
													Stepper={Stepper}
													stepper={stepper}
													allSteps={
														allSteps as Step[]
													}
													valuesRef={
														ref as MutableRefObject<Record<
															string,
															unknown
														> | null>
													}
													step={step}
													content={content}
													footer={footer}
													showNext={
														step.showNext ?? true
													}
													showPrevious={
														step.showPrevious ??
														true
													}
													isLastVisible={isLastVisible(
														step.id,
													)}
												/>
											);
										})
									)}
								</Stepper.Step>
							))}
						</form>
					</FormProvider>
				</StepperFormContext.Provider>
			</Stepper.Navigation>
		</StepVisibilityContext.Provider>
	);
}

function StepPanel<const Steps extends Step[]>({
	Stepper,
	stepper,
	allSteps,
	valuesRef,
	step,
	content,
	showNext = true,
	showPrevious = true,
	showControls = true,
	isLastVisible = false,
	footer,
}: Pick<StepperFormProps<Steps>, "Stepper" | "content"> & {
	stepper: Stepperize.Stepper<Steps>;
	allSteps: Step[];
	valuesRef: MutableRefObject<Record<string, unknown> | null>;
	step: Steps[number];
	showControls?: boolean;
	showNext?: boolean;
	showPrevious?: boolean;
	isLastVisible?: boolean;
	footer?: ReactNode;
}) {
	const form = useFormContext();

	const goToPrev = () => {
		const allLive = form.getValues() as Record<string, unknown>;
		const dirtyFields = form.formState.dirtyFields as Record<
			string,
			unknown
		>;
		const live = Object.fromEntries(
			Object.entries(allLive).filter(([k]) => k in dirtyFields),
		);
		const mergedValues = {
			...(valuesRef.current ?? {}),
			...live,
		} as Record<string, unknown>;
		form.reset(undefined, { keepErrors: false, keepValues: true });
		const prevId = getPrevVisibleStepId(allSteps, step.id, mergedValues);
		if (prevId) {
			stepper.goTo(prevId as Parameters<typeof stepper.goTo>[0]);
		}
	};

	return (
		<Stepper.Panel className="space-y-6">
			{stepper.match(step.id, content)}
			{showControls ? (
				<Stepper.Controls>
					{footer}
					{showPrevious ? (
						<Button
							type="button"
							variant="outline"
							size="icon"
							onClick={goToPrev}
						>
							<Icon icon={faArrowLeft} />
						</Button>
					) : null}
					{showNext ? (
						<Button
							type="submit"
							size="icon"
							disabled={!form.formState.isValid}
							isLoading={form.formState.isSubmitting}
						>
							<Icon icon={faArrowRight} />
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
