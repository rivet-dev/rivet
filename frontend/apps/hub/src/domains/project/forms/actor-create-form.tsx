// import {
// 	FormControl,
// 	FormField,
// 	FormItem,
// 	FormLabel,
// 	FormMessage,
// 	Label,
// 	createSchemaForm,
// } from "@rivet-gg/components";
// import { JsonCode } from "@rivet-gg/components/code-mirror";
// import { type UseFormReturn, useFormContext } from "react-hook-form";
// import z from "zod";
// import { BuildSelect } from "../components/build-select";
// import { RegionSelect } from "../components/region-select";

// import {
// 	Tags as TagsInput,
// 	formSchema as tagsFormSchema,
// } from "@/domains/project/forms/build-tags-form";
// import { useSuspenseQuery } from "@tanstack/react-query";
// import { useState } from "react";
// import { actorBuildsQueryOptions } from "../queries";

// const jsonValid = z.custom<string>((value) => {
// 	try {
// 		JSON.parse(value);
// 		return true;
// 	} catch {
// 		return false;
// 	}
// });

// export const formSchema = z.object({
// 	buildId: z.string().nonempty("Build is required"),
// 	regionId: z.string(),
// 	parameters: jsonValid.optional(),
// 	tags: tagsFormSchema.shape.tags,
// });

// export type FormValues = z.infer<typeof formSchema>;
// export type SubmitHandler = (
// 	values: FormValues,
// 	form: UseFormReturn<FormValues>,
// ) => Promise<void>;

// const { Form, Submit } = createSchemaForm(formSchema);
// export { Form, Submit };

// export const Build = ({
// 	projectNameId,
// 	environmentNameId,
// }: { projectNameId: string; environmentNameId: string }) => {
// 	const { control } = useFormContext<FormValues>();
// 	return (
// 		<FormField
// 			control={control}
// 			name="buildId"
// 			render={({ field }) => (
// 				<FormItem>
// 					<FormLabel>Build</FormLabel>
// 					<FormControl>
// 						<BuildSelect
// 							projectNameId={projectNameId}
// 							environmentNameId={environmentNameId}
// 							onValueChange={field.onChange}
// 							value={field.value}
// 						/>
// 					</FormControl>
// 					<FormMessage />
// 				</FormItem>
// 			)}
// 		/>
// 	);
// };

// export const Region = ({
// 	projectNameId,
// 	environmentNameId,
// }: { projectNameId: string; environmentNameId: string }) => {
// 	const { control } = useFormContext<FormValues>();
// 	return (
// 		<FormField
// 			control={control}
// 			name="regionId"
// 			render={({ field }) => (
// 				<FormItem>
// 					<FormLabel>Region</FormLabel>
// 					<FormControl>
// 						<RegionSelect
// 							projectNameId={projectNameId}
// 							environmentNameId={environmentNameId}
// 							onValueChange={field.onChange}
// 							value={field.value}
// 						/>
// 					</FormControl>
// 					<FormMessage />
// 				</FormItem>
// 			)}
// 		/>
// 	);
// };

// export const Parameters = () => {
// 	const { control } = useFormContext<FormValues>();
// 	return (
// 		<FormField
// 			control={control}
// 			name="parameters"
// 			render={({ field }) => (
// 				<FormItem>
// 					<FormLabel>Parameters</FormLabel>
// 					<FormControl>
// 						<JsonCode {...field} />
// 					</FormControl>
// 					<FormMessage />
// 				</FormItem>
// 			)}
// 		/>
// 	);
// };

// export const Tags = ({
// 	projectNameId,
// 	environmentNameId,
// }: { projectNameId: string; environmentNameId: string }) => {
// 	const { data: builds } = useSuspenseQuery(
// 		actorBuildsQueryOptions({ projectNameId, environmentNameId }),
// 	);

// 	const [tagKeys, setTagKeys] = useState(() =>
// 		Array.from(
// 			new Set(
// 				builds?.flatMap((build) => Object.keys(build.tags)),
// 			).values(),
// 		).map((key) => ({
// 			label: key,
// 			value: key,
// 		})),
// 	);

// 	const [tagValues, setTagValues] = useState(() =>
// 		Array.from(
// 			new Set(
// 				builds?.flatMap((build) => Object.values(build.tags)),
// 			).values(),
// 		).map((key) => ({ label: key, value: key })),
// 	);

// 	return (
// 		<div className="space-y-2">
// 			<Label>Tags</Label>
// 			<TagsInput
// 				keys={tagKeys}
// 				values={tagValues}
// 				onCreateKeyOption={(option) =>
// 					setTagKeys((opts) => [
// 						...opts,
// 						{ label: option, value: option },
// 					])
// 				}
// 				onCreateValueOption={(option) =>
// 					setTagValues((opts) => [
// 						...opts,
// 						{ label: option, value: option },
// 					])
// 				}
// 			/>
// 		</div>
// 	);
// };
