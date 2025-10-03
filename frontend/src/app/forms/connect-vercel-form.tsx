import { faCheck, faSpinnerThird, Icon } from "@rivet-gg/icons";
import { useQuery } from "@tanstack/react-query";
import { AnimatePresence, motion } from "framer-motion";
import { type UseFormReturn, useFormContext } from "react-hook-form";
import z from "zod";
import {
	CodeFrame,
	CodeGroup,
	CodePreview,
	cn,
	createSchemaForm,
	FormControl,
	FormDescription,
	FormField,
	FormItem,
	FormLabel,
	FormMessage,
	Input,
	ScrollArea,
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components";

export const formSchema = z.object({
	plan: z.string(),
	endpoint: z.string().url(),
});

export type FormValues = z.infer<typeof formSchema>;
export type SubmitHandler = (
	values: FormValues,
	form: UseFormReturn<FormValues>,
) => Promise<void>;

const { Form, Submit, SetValue } = createSchemaForm(formSchema);
export { Form, Submit, SetValue };

export const Plan = ({ className }: { className?: string }) => {
	const { control } = useFormContext<FormValues>();
	return (
		<FormField
			control={control}
			name="plan"
			render={({ field }) => (
				<FormItem className={className}>
					<FormLabel className="col-span-1">Vercel Plan</FormLabel>
					<FormControl className="row-start-2">
						<Select
							onValueChange={field.onChange}
							value={field.value}
						>
							<SelectTrigger>
								<SelectValue placeholder="Select your Vercel plan..." />
							</SelectTrigger>
							<SelectContent>
								<SelectItem value="hobby">Hobby</SelectItem>
								<SelectItem value="pro">Pro</SelectItem>
								<SelectItem value="enterprise">
									Enterprise
								</SelectItem>
							</SelectContent>
						</Select>
					</FormControl>
					<FormDescription className="col-span-1">
						Your Vercel plan determines the configuration required
						to properly run your Rivet Engine.
					</FormDescription>
					<FormMessage className="col-span-1" />
				</FormItem>
			)}
		/>
	);
};

const PLAN_TO_MAX_DURATION: Record<string, number> = {
	hobby: 60,
	pro: 300,
	enterprise: 900,
};

const code = ({ plan }: { plan: string }) =>
	`{
	"$schema": "https://openapi.vercel.sh/vercel.json",
	"fluid": false, 	// [!code highlight]
	"functions": {
		"**": {
			"maxDuration": ${PLAN_TO_MAX_DURATION[plan] || 60}, 	// [!code highlight]
		},
	},
}`;

export const Json = () => {
	const { watch } = useFormContext<FormValues>();

	const plan = watch("plan");
	return (
		<div className="space-y-2 mt-2">
			<CodeFrame language="json" title="vercel.json">
				<CodePreview
					className="w-full min-w-0"
					language="json"
					code={code({ plan })}
				/>
			</CodeFrame>
			<FormDescription className="col-span-1">
				<b>Max Duration</b> - The maximum execution time of your
				serverless functions.
				<br />
				<b>Disable Fluid Compute</b> - Rivet has its own intelligent
				load balancing mechanism.
			</FormDescription>
		</div>
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
	const { watch, formState } = useFormContext<FormValues>();
	const endpoint = watch("endpoint");
	const enabled = !!endpoint && z.string().url().safeParse(endpoint).success;

	const { data } = useQuery({
		queryKey: ["vercel-endpoint-check", endpoint],
		queryFn: async () => {
			try {
				const url = new URL("/health", endpoint);
				const response = await fetch(url);
				if (!response.ok) {
					throw new Error("Failed to connect");
				}
				return response.json();
			} catch {
				const url = new URL("/api/rivet/health", endpoint);
				const response = await fetch(url);
				if (!response.ok) {
					throw new Error("Failed to connect");
				}
				return response.json();
			}
		},
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
						"text-center text-muted-foreground text-sm overflow-hidden flex items-center justify-center",
						success && "text-primary-foreground",
					)}
					initial={{ height: 0, opacity: 0.5 }}
					animate={{ height: "4rem", opacity: 1 }}
				>
					{success ? (
						<>
							<Icon
								icon={faCheck}
								className="mr-1.5 text-primary"
							/>{" "}
							Runner successfully connected
						</>
					) : (
						<>
							<Icon
								icon={faSpinnerThird}
								className="mr-1.5 animate-spin"
							/>{" "}
							Waiting for runner to connect...
						</>
					)}
				</motion.div>
			) : null}
		</AnimatePresence>
	);
}
