import {
	faCheck,
	faSpinnerThird,
	faTriangle,
	faTriangleExclamation,
	Icon,
} from "@rivet-gg/icons";
import { useQuery } from "@tanstack/react-query";
import confetti from "canvas-confetti";
import { AnimatePresence, motion } from "framer-motion";
import { useEffect, useState } from "react";
import { type UseFormReturn, useFormContext } from "react-hook-form";
import { useDebounceValue } from "usehooks-ts";
import z from "zod";
import {
	Button,
	CodeFrame,
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
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components";
import { HelpDropdown } from "../help-dropdown";

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
						to properly connect Rivet to Vercel Functions.
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

export const Endpoint = ({
	className,
	placeholder,
}: {
	className?: string;
	placeholder?: string;
}) => {
	const { control } = useFormContext<FormValues>();
	return (
		<FormField
			control={control}
			name="endpoint"
			render={({ field }) => (
				<FormItem className={className}>
					<FormLabel className="col-span-1">Endpoint</FormLabel>
					<FormControl className="row-start-2">
						<Input
							type="url"
							placeholder={
								placeholder || "https://my-rivet-app.vercel.app"
							}
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
	const { watch } = useFormContext<FormValues>();
	const endpoint = watch("endpoint");
	const enabled = !!endpoint && z.string().url().safeParse(endpoint).success;

	const [debounced] = useDebounceValue(endpoint, 300);

	const { isSuccess, isError, isRefetchError, isLoadingError } = useQuery({
		queryKey: ["vercel-endpoint-check", debounced],
		queryFn: async ({ signal }) => {
			try {
				const url = new URL("/health", debounced);
				const response = await fetch(url, { signal });
				if (!response.ok) {
					throw new Error("Failed to connect");
				}
				return response.json();
			} catch {
				const url = new URL("/api/rivet/health", endpoint);
				const response = await fetch(url, { signal });
				if (!response.ok) {
					throw new Error("Failed to connect");
				}
				return response.json();
			}
		},
		enabled,
		retry: 0,
		refetchInterval: 3_000,
	});

	useEffect(() => {
		if (isSuccess) {
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
	}, [isSuccess]);

	return (
		<AnimatePresence>
			{enabled ? (
				<motion.div
					layoutId="msg"
					className={cn(
						"text-center text-muted-foreground text-sm overflow-hidden flex items-center justify-center",
						isSuccess && "text-primary-foreground",
						isError && "text-destructive-foreground",
					)}
					initial={{ height: 0, opacity: 0.5 }}
					animate={{ height: "6rem", opacity: 1 }}
				>
					{isSuccess ? (
						<>
							<Icon
								icon={faCheck}
								className="mr-1.5 text-primary"
							/>{" "}
							Vercel is running with RivetKit v2137
						</>
					) : isError || isRefetchError || isLoadingError ? (
						<>
							<Icon
								icon={faTriangleExclamation}
								className="mr-1.5 text-destructive"
							/>{" "}
							Health check failed, verify the URL matches.
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
				</motion.div>
			) : null}
		</AnimatePresence>
	);
}

export function NeedHelp() {
	const [open, setOpen] = useState(false);

	useEffect(() => {
		const timeout = setTimeout(() => {
			setOpen(true);
		}, 10000);
		return () => clearTimeout(timeout);
	}, []);

	if (!open) return null;

	return (
		<HelpDropdown>
			<Button variant="ghost">Need help?</Button>
		</HelpDropdown>
	);
}
