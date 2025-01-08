import {
	FormControl,
	FormField,
	FormItem,
	FormLabel,
	FormMessage,
	Input,
	createSchemaForm,
} from "@rivet-gg/components";
import { useSuspenseQuery } from "@tanstack/react-query";
import { type UseFormReturn, useFormContext } from "react-hook-form";
import Turnstile from "react-turnstile";
import z from "zod";
import { bootstrapCaptchaQueryOptions } from "../queries/bootstrap";

export const formSchema = z.object({
	email: z.string().email(),
	captcha: z.string(),
});

export type FormValues = z.infer<typeof formSchema>;
export type SubmitHandler = (
	values: FormValues,
	form: UseFormReturn<FormValues>,
) => Promise<void>;

const { Form, Submit } = createSchemaForm(formSchema);
export { Form, Submit };

export const Email = () => {
	const { control } = useFormContext<FormValues>();
	return (
		<FormField
			control={control}
			name="email"
			render={({ field }) => (
				<FormItem>
					<FormLabel>Email</FormLabel>
					<FormControl>
						<Input
							placeholder="Enter your email here..."
							{...field}
						/>
					</FormControl>
					<FormMessage />
				</FormItem>
			)}
		/>
	);
};

export const Captcha = () => {
	const { control } = useFormContext<FormValues>();
	const { data } = useSuspenseQuery(bootstrapCaptchaQueryOptions());

	return (
		<FormField
			control={control}
			name="captcha"
			render={({ field }) => (
				<FormItem>
					<FormControl>
						{data.turnstile?.siteKey ? (
							<Turnstile
								refreshExpired="auto"
								retry="auto"
								sitekey={data.turnstile.siteKey}
								onVerify={(token) => {
									field.onChange({
										target: { value: token },
									});
								}}
							/>
						) : null}
					</FormControl>
					<FormMessage />
				</FormItem>
			)}
		/>
	);
};
