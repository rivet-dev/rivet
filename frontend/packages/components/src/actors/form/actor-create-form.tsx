import {
	FormControl,
	FormField,
	FormItem,
	FormLabel,
	FormMessage,
	Label,
	createSchemaForm,
} from "@rivet-gg/components";
import { JsonCode } from "@rivet-gg/components/code-mirror";
import { type UseFormReturn, useFormContext } from "react-hook-form";
import z from "zod";

import {
	Tags as TagsInput,
	formSchema as tagsFormSchema,
} from "./build-tags-form";
import { RegionSelect } from "../region-select";
import { useAtomValue, useSetAtom } from "jotai";
import {
	actorCustomTagKeys,
	actorCustomTagValues,
	actorTagKeysAtom,
	actorTagValuesAtom,
} from "../actor-context";
import { BuildSelect } from "../build-select";

const jsonValid = z.custom<string>((value) => {
	try {
		JSON.parse(value);
		return true;
	} catch {
		return false;
	}
});

export const formSchema = z.object({
	buildId: z.string().nonempty("Build is required"),
	regionId: z.string(),
	parameters: jsonValid.optional(),
	tags: tagsFormSchema.shape.tags,
});

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
			name="buildId"
			render={({ field }) => (
				<FormItem>
					<FormLabel>Build</FormLabel>
					<FormControl>
						<BuildSelect
							onlyCurrent
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

export const Region = () => {
	const { control } = useFormContext<FormValues>();

	return (
		<FormField
			control={control}
			name="regionId"
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

export const Parameters = () => {
	const { control } = useFormContext<FormValues>();
	return (
		<FormField
			control={control}
			name="parameters"
			render={({ field }) => (
				<FormItem>
					<FormLabel>Parameters</FormLabel>
					<FormControl>
						<JsonCode {...field} />
					</FormControl>
					<FormMessage />
				</FormItem>
			)}
		/>
	);
};

export const Tags = () => {
	const setValues = useSetAtom(actorCustomTagValues);
	const setKeys = useSetAtom(actorCustomTagKeys);

	const keys = useAtomValue(actorTagKeysAtom);
	const values = useAtomValue(actorTagValuesAtom);

	return (
		<div className="space-y-2">
			<Label>Tags</Label>
			<TagsInput
				keys={keys.map((key) => ({
					label: key,
					value: key,
				}))}
				values={values.map((value) => ({
					label: value,
					value: value,
				}))}
				onCreateKeyOption={(value) => {
					setKeys((old) =>
						Array.from(new Set([...old, value]).values()),
					);
				}}
				onCreateValueOption={(value) => {
					setValues((old) =>
						Array.from(new Set([...old, value]).values()),
					);
				}}
			/>
		</div>
	);
};
