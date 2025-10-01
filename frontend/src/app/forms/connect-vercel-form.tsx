import { faCopy, Icon } from "@rivet-gg/icons";
import { useQuery } from "@tanstack/react-query";
import { useParams } from "@tanstack/react-router";
import { type UseFormReturn, useFormContext } from "react-hook-form";
import z from "zod";
import {
	Button,
	CodePreview,
	CopyButton,
	createSchemaForm,
	FormControl,
	FormField,
	FormItem,
	FormLabel,
	FormMessage,
	Input,
	Label,
	ScrollArea,
} from "@/components";
import { useCloudDataProvider } from "@/components/actors";

export const formSchema = z.object({
	name: z
		.string()
		.max(16)
		.refine((value) => value.trim() !== "" && value.trim() === value, {
			message: "Name cannot be empty or contain whitespaces",
		}),
	endpoint: z.string().url(),
});

export type FormValues = z.infer<typeof formSchema>;
export type SubmitHandler = (
	values: FormValues,
	form: UseFormReturn<FormValues>,
) => Promise<void>;

const { Form, Submit, SetValue } = createSchemaForm(formSchema);
export { Form, Submit, SetValue };

export const Name = ({ className }: { className?: string }) => {
	const { control } = useFormContext<FormValues>();
	return (
		<FormField
			control={control}
			name="name"
			render={({ field }) => (
				<FormItem className={className}>
					<FormLabel className="col-span-1">Name</FormLabel>
					<FormControl className="row-start-2">
						<Input
							placeholder="Enter a runner name..."
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

export const Endpoint = ({ className }: { className?: string }) => {
	const { control } = useFormContext<FormValues>();
	return (
		<FormField
			control={control}
			name="endpoint"
			render={({ field }) => (
				<FormItem className={className}>
					<FormLabel className="col-span-1">
						Functions Endpoint
					</FormLabel>
					<FormControl className="row-start-2">
						<Input
							placeholder="https://your-application.vercel.app"
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

const code = ({
	token,
	name,
}: {
	token: string;
	name: string;
}) => `import { registry } from "./registry";

registry.runServer({ 
	token: "${token}",
	name: "${name}",
});`;

export function Preview() {
	const params = useParams({
		from: "/_context/_cloud/orgs/$organization/projects/$project/ns/$namespace",
	});
	const { data } = useQuery(
		useCloudDataProvider().currentOrgProjectNamespaceQueryOptions(params),
	);

	const { watch } = useFormContext<FormValues>();
	const name = watch("name");
	return (
		<div className="space-y-2">
			<Label>Code</Label>
			<div className="text-xs border rounded-md p-2 relative w-full">
				<ScrollArea>
					<CodePreview
						className="w-full min-w-0"
						language="typescript"
						code={code({
							token: data?.access.token || "<TOKEN>",
							name,
						})}
					/>
				</ScrollArea>

				<CopyButton
					value={code({
						token: data?.access.token || "<TOKEN>",
						name,
					})}
				>
					<Button
						variant="secondary"
						size="icon-sm"
						className="absolute top-1 right-2"
					>
						<Icon icon={faCopy} />
					</Button>
				</CopyButton>
			</div>
		</div>
	);
}
