import { faCheck, faSpinnerThird, Icon } from "@rivet-gg/icons";
import { useQuery } from "@tanstack/react-query";
import { useParams } from "@tanstack/react-router";
import { AnimatePresence, motion } from "framer-motion";
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
import {
	useCloudDataProvider,
	useEngineCompatDataProvider,
} from "@/components/actors";

export const formSchema = z.object({
	name: z
		.string()
		.max(16)
		.refine((value) => value.trim() !== "" && value.trim() === value, {
			message: "Name cannot be empty or contain whitespaces",
		}),
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

export function ConnectionCheck() {
	const params = useParams({
		from: "/_context/_cloud/orgs/$organization/projects/$project/ns/$namespace",
	});
	const { watch } = useFormContext<FormValues>();
	const name = watch("name");

	const { data: namespace } = useQuery(
		useCloudDataProvider().currentOrgProjectNamespaceQueryOptions(params),
	);

	const enabled = !!name && !!namespace?.access.engineNamespaceName;

	const { data } = useQuery({
		...useEngineCompatDataProvider().runnerByNameQueryOptions({
			namespace: namespace?.access.engineNamespaceName || "",
			runnerName: name,
		}),
		enabled,
		refetchInterval: 1000,
	});

	const success = !!data;

	return (
		<AnimatePresence>
			{enabled ? (
				<motion.div
					layoutId="msg"
					className={cn(
						"text-center text-muted-foreground text-xs overflow-hidden flex items-center justify-center",
						success && "text-primary-foreground",
					)}
					initial={{ height: 0, opacity: 0.5 }}
					animate={{ height: "4rem", opacity: 1 }}
				>
					{success ? (
						<>
							<Icon
								icon={faCheck}
								className="mr-1 text-primary"
							/>{" "}
							Runner successfully connected
						</>
					) : (
						<>
							<Icon
								icon={faSpinnerThird}
								className="mr-1 animate-spin"
							/>{" "}
							Waiting for runner to connect...
						</>
					)}
				</motion.div>
			) : null}
		</AnimatePresence>
	);
}

export { Preview } from "./connect-vercel-form";
