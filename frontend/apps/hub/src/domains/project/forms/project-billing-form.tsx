import {
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
	createSchemaForm,
	formatCurrency,
} from "@rivet-gg/components";
import { useEffect } from "react";
import { type UseFormReturn, useFormContext } from "react-hook-form";
import z from "zod";

export const hardwareTierValues = [
	{ value: "1/8", label: "1/8 core", multiplier: 1 / 8 },
	{ value: "1/4", label: "1/4 core", multiplier: 1 / 4 },
	{ value: "1/2", label: "1/2 core", multiplier: 1 / 2 },
	{ value: "1", label: "1 core", multiplier: 1 },
	{ value: "2", label: "2 cores", multiplier: 2 },
	{ value: "4", label: "4 cores", multiplier: 4 },
];

const PRICE_PER_CORE = 22.76;

const MAX_CORES = 32768;

export const formSchema = z.object({
	hardwareTier: z.string(),
	capacity: z.array(
		z.object({
			regionId: z.string(),
			cores: z.coerce.number().min(0).max(MAX_CORES),
		}),
	),
});

export type FormValues = z.infer<typeof formSchema>;
export type SubmitHandler = (
	values: FormValues,
	form: UseFormReturn<FormValues>,
) => Promise<void>;

const { Form, Submit, Reset } = createSchemaForm(formSchema);
export { Form, Submit, Reset };

export const HardwareTier = () => {
	const { control } = useFormContext<FormValues>();
	return (
		<FormField
			control={control}
			name="hardwareTier"
			render={({ field }) => (
				<FormItem>
					<FormLabel>Dedicated hardware</FormLabel>
					<Select
						onValueChange={field.onChange}
						defaultValue={`${field.value}`}
					>
						<FormControl>
							<SelectTrigger>
								<SelectValue placeholder="Select dedicated hardware" />
							</SelectTrigger>
						</FormControl>
						<SelectContent>
							{hardwareTierValues.map((item) => (
								<SelectItem key={item.value} value={item.value}>
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

export const HardwareMultiplier = () => {
	const { watch, setValue } = useFormContext<FormValues>();

	useEffect(() => {
		let lastHardwareTier: string | undefined;
		const { unsubscribe } = watch((value) => {
			if (lastHardwareTier === value.hardwareTier) return;
			if (!value.hardwareTier || !value.capacity) return;
			const lastHardwareTierValue = hardwareTierValues.find(
				(item) => item.value === lastHardwareTier,
			);
			const currentHardwareTierValue = hardwareTierValues.find(
				(item) => item.value === value.hardwareTier,
			);

			lastHardwareTier = value.hardwareTier;
			if (!currentHardwareTierValue || !lastHardwareTierValue) return;

			setValue(
				"capacity",
				value.capacity?.map((item) => ({
					cores: Math.ceil(
						((item?.cores ?? 1) *
							lastHardwareTierValue.multiplier) /
							currentHardwareTierValue.multiplier,
					),
					regionId: item?.regionId ?? "",
				})) ?? [],
			);

			lastHardwareTier = value.hardwareTier;
		});
		return () => unsubscribe();
	}, [setValue, watch]);

	return null;
};

interface CapacityProps {
	index: number;
	regionId: string;
}

export const Capacity = ({ index, regionId }: CapacityProps) => {
	const { control, register } = useFormContext<FormValues>();
	return (
		<>
			<FormField
				control={control}
				name={`capacity.${index}.cores`}
				render={({ field }) => (
					<FormItem>
						<Input
							type="number"
							placeholder="0"
							min={0}
							max={MAX_CORES}
							{...field}
						/>
						<FormMessage />
					</FormItem>
				)}
			/>
			<input
				type="hidden"
				{...register(`capacity.${index}.regionId`, { value: regionId })}
			/>
		</>
	);
};

export const HardwareTierValueLabel = () => {
	const { watch } = useFormContext<FormValues>();

	const hardwareTier = watch("hardwareTier");

	const hardwareTierValue = hardwareTierValues.find(
		(item) => item.value === hardwareTier,
	);
	return hardwareTierValue?.label ?? "1 core";
};

interface CapacityValueProps {
	children: (value: { regionId: string; cores: number }[]) => React.ReactNode;
}

export const CapacityValue = ({ children }: CapacityValueProps) => {
	const { watch } = useFormContext<FormValues>();
	const capacity = watch("capacity");

	return <>{children(capacity)}</>;
};

interface RegionTotalPriceProps {
	index: number;
}

export const RegionTotalPrice = ({ index }: RegionTotalPriceProps) => {
	const { watch } = useFormContext<FormValues>();
	const cores = watch(`capacity.${index}.cores`);
	const hardwareTier = watch("hardwareTier");

	const hardwareTierValue =
		hardwareTierValues.find((item) => item.value === hardwareTier)
			?.multiplier ?? 1;

	return <>{formatCurrency(cores * PRICE_PER_CORE * hardwareTierValue)}</>;
};

export const TotalPrice = () => {
	const { watch } = useFormContext<FormValues>();
	const capacity = watch("capacity");
	const hardwareTier = watch("hardwareTier");

	const total = capacity.reduce((acc, item) => {
		const hardwareTierValue =
			hardwareTierValues.find((tier) => tier.value === hardwareTier)
				?.multiplier ?? 1;
		return acc + item.cores * PRICE_PER_CORE * hardwareTierValue;
	}, 0);

	return <>{formatCurrency(total)}</>;
};
