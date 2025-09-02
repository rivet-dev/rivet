import { useInfiniteQuery } from "@tanstack/react-query";
import { useEffect, useRef } from "react";
import { type UseFormReturn, useFormContext } from "react-hook-form";
import z from "zod";
import { CodePreview, Input, Label } from "@/components";
import {
	type NamespaceNameId,
	runnerNamesQueryOptions,
} from "@/queries/manager-engine";
import { JsonCode } from "../../code-mirror";
import { createSchemaForm } from "../../lib/create-schema-form";
import {
	FormControl,
	FormDescription,
	FormField,
	FormItem,
	FormLabel,
	FormMessage,
} from "../../ui/form";
import { BuildSelect } from "../build-select";
import { CrashPolicySelect } from "../crash-policy-select";
import { CrashPolicy as CrashPolicyEnum } from "../queries";
import { RegionSelect } from "../region-select";
import { RunnerSelect } from "../runner-select";

const jsonValid = z.custom<string>(
	(value) => {
		if (value.trim() === "") return true;
		try {
			JSON.parse(value);
			return true;
		} catch {
			return false;
		}
	},
	{ fatal: true, message: "Must be valid JSON" },
);

export const formSchema = z
	.object({
		name: z.string().nonempty("Build is required"),
		// regionId: z.string(),
		key: z.string(),
		input: jsonValid.optional(),
		// tags: tagsFormSchema.shape.tags,
	})
	.and(
		__APP_TYPE__ === "engine"
			? z.object({
					region: z.string(),
					runnerNameSelector: z.string(),
					crashPolicy: z.nativeEnum(CrashPolicyEnum),
				})
			: z.object({}),
	);

export type FormValues = z.infer<typeof formSchema>;
export type SubmitHandler = (
	values: FormValues,
	form: UseFormReturn<FormValues>,
) => Promise<void>;

const { Form, Submit } = createSchemaForm(formSchema);
export { Form, Submit };

export const Build = () => {
	const { control } = useFormContext<FormValues>();
	return (
		<FormField
			control={control}
			name="name"
			render={({ field }) => (
				<FormItem>
					<FormLabel>Name</FormLabel>
					<FormControl>
						<BuildSelect
							onValueChange={field.onChange}
							value={field.value}
						/>
					</FormControl>
					<FormDescription>
						Used to differentiate between different actor types.
					</FormDescription>
					<FormMessage />
				</FormItem>
			)}
		/>
	);
};

export const CrashPolicy = () => {
	const { control } = useFormContext<FormValues>();

	return (
		<FormField
			control={control}
			name="crashPolicy"
			render={({ field }) => (
				<FormItem>
					<FormLabel>Crash Policy</FormLabel>
					<FormControl>
						<CrashPolicySelect
							onValueChange={field.onChange}
							value={field.value}
						/>
					</FormControl>
					<FormDescription>
						Determines the behavior of the actor on crash.
					</FormDescription>
					<FormMessage />
				</FormItem>
			)}
		/>
	);
};

export const Keys = () => {
	const { control } = useFormContext<FormValues>();
	return (
		<FormField
			control={control}
			name="key"
			render={({ field }) => (
				<FormItem>
					<FormLabel>Key</FormLabel>
					<FormControl>
						<Input {...field} className="font-mono-console" />
					</FormControl>
					<FormDescription>Identifier for the Actor.</FormDescription>
					<FormMessage />
				</FormItem>
			)}
		/>
	);
};

export const JsonInput = () => {
	const { control } = useFormContext<FormValues>();
	return (
		<FormField
			control={control}
			name="input"
			render={({ field }) => (
				<FormItem>
					<FormLabel>Input</FormLabel>
					<FormControl>
						<JsonCode
							minHeight="5rem"
							onChange={field.onChange}
							value={field.value}
						/>
					</FormControl>
					<FormDescription>
						Optional JSON object that will be passed to the Actor as
						input.
					</FormDescription>
					<FormMessage />
				</FormItem>
			)}
		/>
	);
};

export const RunnerNameSelector = ({
	namespace,
}: {
	namespace: NamespaceNameId;
}) => {
	const { control } = useFormContext<FormValues>();

	return (
		<FormField
			control={control}
			name="runnerNameSelector"
			render={({ field }) => (
				<FormItem>
					<FormLabel>Runner</FormLabel>
					<FormControl>
						<RunnerSelect
							namespace={namespace}
							onValueChange={field.onChange}
							value={field.value}
						/>
					</FormControl>
					<FormDescription>
						Runner name selector for the actor. This is used to
						select which runner the actor will run on.
					</FormDescription>
					<FormMessage />
				</FormItem>
			)}
		/>
	);
};

export const ActorPreview = () => {
	const { watch } = useFormContext<FormValues>();

	const [name, key] = watch(["name", "key"]);

	return (
		<div className="space-y-2">
			<Label>Code</Label>
			<div className="text-xs border rounded-md p-2">
				<CodePreview
					code={`client.${name}.getOrCreate(${JSON.stringify(key)})`}
					language="typescript"
				/>
			</div>
			<p className={"text-sm text-muted-foreground"}>
				You can use above code snippet to get or create the actor in
				your application. For more information, see the{" "}
				<a
					href="https://www.rivet.gg/docs/actors/clients/#client-setup"
					target="_blank"
					rel="noopener noreferrer"
					className="underline"
				>
					documentation
				</a>
				.
			</p>
		</div>
	);
};

export const PrefillRunnerName = ({
	namespace,
}: {
	namespace: NamespaceNameId;
}) => {
	const prefilled = useRef(false);
	const { watch } = useFormContext<FormValues>();

	const { data = [], isSuccess } = useInfiniteQuery(
		runnerNamesQueryOptions({ namespace }),
	);

	const watchedValue = watch("runnerNameSelector");

	const { setValue } = useFormContext<FormValues>();

	useEffect(() => {
		if (
			data.length > 0 &&
			isSuccess &&
			!watchedValue &&
			!prefilled.current
		) {
			setValue("runnerNameSelector", data[0]);
			prefilled.current = true;
		}
	}, [data, setValue, isSuccess, watchedValue]);

	return null;
};

export const Region = () => {
	const { control } = useFormContext<FormValues>();

	return (
		<FormField
			control={control}
			name="region"
			render={({ field }) => (
				<FormItem>
					<FormLabel>Region</FormLabel>
					<FormControl>
						<RegionSelect
							value={field.value}
							onValueChange={field.onChange}
						/>
					</FormControl>
					<FormDescription>
						The region where the Actor will be deployed.
					</FormDescription>
					<FormMessage />
				</FormItem>
			)}
		/>
	);
};

// export const Tags = () => {
// 	// const setValues = useSetAtom(actorCustomTagValues);
// 	// const setKeys = useSetAtom(actorCustomTagKeys);

// 	const { data: tags = [] } = useInfiniteQuery(
// 		useManagerQueries().actorsTagsQueryOptions(),
// 	);

// 	const keys = useMemo(() => {
// 		return Array.from(
// 			new Set(tags.flatMap((tag) => Object.keys(tag))),
// 		).sort();
// 	}, [tags]);
// 	const values = useMemo(() => {
// 		return Array.from(
// 			new Set(tags.flatMap((tag) => Object.values(tag))),
// 		).sort();
// 	}, [tags]);

// 	return (
// 		<div className="space-y-2">
// 			<Label>Tags</Label>
// 			<TagsInput
// 				keys={keys.map((key) => ({
// 					label: key,
// 					value: key,
// 				}))}
// 				values={values.map((value) => ({
// 					label: value,
// 					value: value,
// 				}))}
// 				onCreateKeyOption={(value) => {
// 					// setKeys((old) =>
// 					// 	Array.from(new Set([...old, value]).values()),
// 					// );
// 				}}
// 				onCreateValueOption={(value) => {
// 					// setValues((old) =>
// 					// 	Array.from(new Set([...old, value]).values()),
// 					// );
// 				}}
// 			/>
// 		</div>
// 	);
// };
