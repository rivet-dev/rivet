import {
	Flex,
	FormControl,
	FormDescription,
	FormField,
	FormItem,
	FormLabel,
	FormMessage,
	Grid,
	Link,
	Slider,
	SmallText,
	createSchemaForm,
} from "@rivet-gg/components";
import { type UseFormReturn, useFormContext } from "react-hook-form";
import z from "zod";

export const AUTOSCALING_VALUE_MAP = [
	{ value: 0.25, memory: 1, vcpu: 0.25 },
	{ value: 0.5, memory: 2, vcpu: 0.5 },
	{ value: 1, memory: 4, vcpu: 1 },
	{ value: 2, memory: 8, vcpu: 2 },
	{ value: 3, memory: 12, vcpu: 3 },
	{ value: 4, memory: 16, vcpu: 4 },
	{ value: 5, memory: 20, vcpu: 5 },
	{ value: 6, memory: 24, vcpu: 6 },
	{ value: 7, memory: 28, vcpu: 7 },
	{ value: 8, memory: 32, vcpu: 8 },
];

export const AUTOSCALING_VALUE_TO_INDEX_MAP = Object.fromEntries(
	AUTOSCALING_VALUE_MAP.map(({ value }, index) => [value, index]),
);

export const formSchema = z.object({
	autoscalling: z.object({
		min: z.coerce
			.number()
			.min(0)
			.max(AUTOSCALING_VALUE_MAP.length - 1),
		max: z.coerce
			.number()
			.min(0)
			.max(AUTOSCALING_VALUE_MAP.length - 1),
	}),
});

export type FormValues = z.infer<typeof formSchema>;
export type SubmitHandler = (
	values: FormValues,
	form: UseFormReturn<FormValues>,
) => Promise<void>;

const { Form, Submit } = createSchemaForm(formSchema);
export { Form, Submit };

const AutoscalingLabel = () => {
	const { watch } = useFormContext<FormValues>();
	const autoscaling = watch("autoscalling");

	const min = AUTOSCALING_VALUE_MAP[autoscaling.min];
	const max = AUTOSCALING_VALUE_MAP[autoscaling.max];
	return (
		<Flex justify="between" items="center" mb="4" gap="4">
			<SmallText>
				Min: {min.vcpu} vCPU, {min.memory} GB of RAM
			</SmallText>
			<SmallText>
				Max: {max.vcpu} vCPU, {max.memory} GB of RAM
			</SmallText>
		</Flex>
	);
};

export const Autoscaling = () => {
	const { control } = useFormContext<FormValues>();
	return (
		<FormField
			control={control}
			name="autoscalling"
			render={({ field }) => (
				<FormItem>
					<FormLabel>Autoscaling</FormLabel>
					<FormControl>
						<Flex direction="col">
							<AutoscalingLabel />
							<Flex direction="col" p="2">
								<Slider
									{...field}
									onValueChange={(value) =>
										field.onChange({
											min: value[0],
											max: value[1],
										})
									}
									step={1}
									min={0}
									max={AUTOSCALING_VALUE_MAP.length - 1}
									value={[field.value.min, field.value.max]}
									defaultValue={[
										field.value.min,
										field.value.max,
									]}
								/>
								<Grid columns="10" mt="4" mx="-2">
									{AUTOSCALING_VALUE_MAP.map(
										({ value }, index) => (
											<SmallText
												textAlign={"center"}
												key={value}
											>
												{value}
											</SmallText>
										),
									)}
								</Grid>
							</Flex>
						</Flex>
					</FormControl>

					<FormDescription>
						Need more resources?{" "}
						<Link
							href="https://rivet.gg/support"
							target="_blank"
							rel="noreferrer"
						>
							Contact support.
						</Link>
					</FormDescription>
					<FormMessage />
				</FormItem>
			)}
		/>
	);
};
