import { faCheck, faSpinnerThird, Icon } from "@rivet-gg/icons";
import { useInfiniteQuery } from "@tanstack/react-query";
import confetti from "canvas-confetti";
import { useEffect, useRef } from "react";
import { type UseFormReturn, useFormContext } from "react-hook-form";
import z from "zod";
import {
	cn,
	createSchemaForm,
	FormControl,
	FormField,
	FormItem,
	FormLabel,
	FormMessage,
	Input,
} from "@/components";
import { useEngineCompatDataProvider } from "@/components/actors";
import { RegionSelect } from "@/components/actors/region-select";

export const formSchema = z.object({
	runnerName: z.string().default("rivetkit"),
	datacenter: z.string().default("auto"),
});

export type FormValues = z.infer<typeof formSchema>;
export type SubmitHandler = (
	values: FormValues,
	form: UseFormReturn<FormValues>,
) => Promise<void>;

const { Form, Submit, SetValue } = createSchemaForm(formSchema);
export { Form, Submit, SetValue };

export const RunnerName = function RunnerName() {
	const { control } = useFormContext<FormValues>();
	return (
		<FormField
			control={control}
			name="runnerName"
			render={({ field }) => (
				<FormItem>
					<FormLabel className="col-span-1">Runner Name</FormLabel>
					<FormControl className="row-start-2">
						<Input type="text" {...field} />
					</FormControl>
					<FormMessage className="col-span-1" />
				</FormItem>
			)}
		/>
	);
};

export const Datacenter = function Datacenter() {
	const { control } = useFormContext<FormValues>();
	return (
		<FormField
			control={control}
			name="datacenter"
			render={({ field }) => (
				<FormItem>
					<FormLabel>Region</FormLabel>
					<FormControl>
						<RegionSelect
							onValueChange={field.onChange}
							value={field.value}
						/>
					</FormControl>
					<FormMessage />
				</FormItem>
			)}
		/>
	);
};

export const ConnectionCheck = function ConnectionCheck() {
	const { data } = useInfiniteQuery({
		...useEngineCompatDataProvider().runnersQueryOptions(),
		refetchInterval: 1000,
		maxPages: 9999,
		select: (data) =>
			data.pages.reduce((acc, page) => acc + page.runners.length, 0),
	});

	const lastCount = useRef(data);

	useEffect(() => {
		lastCount.current = data;
	}, [data]);

	const success =
		data !== undefined && data > 0 && data !== lastCount.current;

	useEffect(() => {
		if (success) {
			confetti({
				angle: 60,
				spread: 55,
				origin: { x: 0 },
			});
			confetti({
				angle: 120,
				spread: 55,
				origin: { x: 1 },
			});
		}
	}, [success]);

	return (
		<div
			className={cn(
				"text-center h-24 text-muted-foreground text-sm overflow-hidden flex items-center justify-center",
				success && "text-primary-foreground",
			)}
		>
			{success ? (
				<>
					<Icon icon={faCheck} className="mr-1.5 text-primary" />{" "}
					Runner successfully connected
				</>
			) : (
				<div className="flex flex-col items-center gap-2">
					<div className="flex items-center">
						<Icon
							icon={faSpinnerThird}
							className="mr-1.5 animate-spin"
						/>{" "}
						Waiting for Runner to connect...
					</div>
				</div>
			)}
		</div>
	);
};
