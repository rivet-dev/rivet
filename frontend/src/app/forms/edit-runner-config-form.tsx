import { type UseFormReturn, useFormContext } from "react-hook-form";
import z from "zod";
import {
	createSchemaForm,
	FormControl,
	FormField,
	FormItem,
	FormLabel,
	FormMessage,
	Input,
} from "@/components";

export const formSchema = z.object({
	url: z.string().url(),
	maxRunners: z.number().positive(),
	minRunners: z.number().positive(),
	requestLifespan: z.number().positive(),
	runnersMargin: z.number().positive(),
	slotsPerRunner: z.number().positive(),
});

export type FormValues = z.infer<typeof formSchema>;
export type SubmitHandler = (
	values: FormValues,
	form: UseFormReturn<FormValues>,
) => Promise<void>;

const { Form, Submit, SetValue } = createSchemaForm(formSchema);
export { Form, Submit, SetValue };

export const Url = ({ className }: { className?: string }) => {
	const { control } = useFormContext<FormValues>();
	return (
		<FormField
			control={control}
			name="url"
			render={({ field }) => (
				<FormItem className={className}>
					<FormLabel className="col-span-1">Url</FormLabel>
					<FormControl className="row-start-2">
						<Input
							placeholder="https://your-rivet-runner"
							maxLength={25}
							{...field}
						/>
					</FormControl>
					<FormMessage className="col-span-1" />
				</FormItem>
			)}
		/>
	);
};

export const MinRunners = ({ className }: { className?: string }) => {
	const { control } = useFormContext<FormValues>();
	return (
		<FormField
			control={control}
			name="url"
			render={({ field }) => (
				<FormItem className={className}>
					<FormLabel className="col-span-1">Min Runners</FormLabel>
					<FormControl className="row-start-2">
						<Input type="number" {...field} />
					</FormControl>
					<FormMessage className="col-span-1" />
				</FormItem>
			)}
		/>
	);
};

export const MaxRunners = ({ className }: { className?: string }) => {
	const { control } = useFormContext<FormValues>();
	return (
		<FormField
			control={control}
			name="maxRunners"
			render={({ field }) => (
				<FormItem className={className}>
					<FormLabel className="col-span-1">Max Runners</FormLabel>
					<FormControl className="row-start-2">
						<Input type="number" {...field} />
					</FormControl>
					<FormMessage className="col-span-1" />
				</FormItem>
			)}
		/>
	);
};

export const RequestLifespan = ({ className }: { className?: string }) => {
	const { control } = useFormContext<FormValues>();
	return (
		<FormField
			control={control}
			name="requestLifespan"
			render={({ field }) => (
				<FormItem className={className}>
					<FormLabel className="col-span-1">
						Request Lifespan
					</FormLabel>
					<FormControl className="row-start-2">
						<Input type="number" {...field} />
					</FormControl>
					<FormMessage className="col-span-1" />
				</FormItem>
			)}
		/>
	);
};

export const RunnersMargin = ({ className }: { className?: string }) => {
	const { control } = useFormContext<FormValues>();
	return (
		<FormField
			control={control}
			name="runnersMargin"
			render={({ field }) => (
				<FormItem className={className}>
					<FormLabel className="col-span-1">Runners Margin</FormLabel>
					<FormControl className="row-start-2">
						<Input type="number" {...field} />
					</FormControl>
					<FormMessage className="col-span-1" />
				</FormItem>
			)}
		/>
	);
};

export const SlotsPerRunner = ({ className }: { className?: string }) => {
	const { control } = useFormContext<FormValues>();
	return (
		<FormField
			control={control}
			name="slotsPerRunner"
			render={({ field }) => (
				<FormItem className={className}>
					<FormLabel className="col-span-1">
						Slots Per Runner
					</FormLabel>
					<FormControl className="row-start-2">
						<Input type="number" {...field} />
					</FormControl>
					<FormMessage className="col-span-1" />
				</FormItem>
			)}
		/>
	);
};
