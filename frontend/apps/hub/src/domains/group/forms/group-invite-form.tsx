import {
	Flex,
	FormControl,
	FormField,
	FormItem,
	FormLabel,
	FormMessage,
	Input,
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
	Switch,
	createSchemaForm,
	timing,
} from "@rivet-gg/components";
import { type UseFormReturn, useFormContext } from "react-hook-form";
import z from "zod";

const expirationValues = [
	{ label: "30 minutes", value: timing.minutes(30) },
	{ label: "1 hour", value: timing.hours(1) },
	{ label: "6 hours", value: timing.hours(6) },
	{ label: "12 hours", value: timing.hours(12) },
	{ label: "1 day", value: timing.days(1) },
	{ label: "1 week", value: timing.days(7) },
	{ label: "1 month", value: timing.days(30) },
	{ label: "never", value: 0 },
];

export const formSchema = z.object({
	expTime: z.coerce.number(),
	isInfinite: z.boolean(),
	usageCount: z.coerce.number().min(1).max(5000).default(10),
});

export type FormValues = z.infer<typeof formSchema>;
export type SubmitHandler = (
	values: FormValues,
	form: UseFormReturn<FormValues>,
) => Promise<void>;

const { Form, Submit } = createSchemaForm(formSchema);
export { Form, Submit };

export const ExpirationTime = () => {
	const { control } = useFormContext<FormValues>();
	return (
		<FormField
			control={control}
			name="expTime"
			render={({ field }) => (
				<FormItem>
					<FormLabel>Expire after</FormLabel>
					<Select
						onValueChange={field.onChange}
						defaultValue={`${field.value}`}
					>
						<FormControl>
							<SelectTrigger>
								<SelectValue placeholder="Select expiration" />
							</SelectTrigger>
						</FormControl>
						<SelectContent>
							{expirationValues.map((item) => (
								<SelectItem
									key={item.value}
									value={`${item.value}`}
								>
									{item.label}
								</SelectItem>
							))}
						</SelectContent>
					</Select>
					<FormMessage />
				</FormItem>
			)}
		/>
	);
};

export const Infinite = () => {
	const { control } = useFormContext<FormValues>();
	return (
		<FormField
			control={control}
			name="isInfinite"
			render={({ field }) => (
				<FormItem>
					<Flex items="center" gap="4">
						<Switch
							checked={field.value}
							onCheckedChange={field.onChange}
						/>
						<FormLabel>Infinite use</FormLabel>
					</Flex>
					<FormMessage />
				</FormItem>
			)}
		/>
	);
};

export const UsageCount = () => {
	const { control, watch } = useFormContext<FormValues>();

	const isInfinite = watch("isInfinite");

	if (isInfinite) {
		return null;
	}

	return (
		<FormField
			control={control}
			name="usageCount"
			render={({ field }) => (
				<FormItem>
					<FormLabel>Max number of uses</FormLabel>
					<FormControl>
						<Input
							type="number"
							placeholder="10"
							min={1}
							step={1}
							{...field}
						/>
					</FormControl>
					<FormMessage />
				</FormItem>
			)}
		/>
	);
};
