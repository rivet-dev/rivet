import { faBug, faConciergeBell, Icon } from "@rivet-gg/icons";
import { type UseFormReturn, useFormContext } from "react-hook-form";
import z from "zod";
import { createSchemaForm } from "../lib/create-schema-form";
import {
	FormControl,
	FormField,
	FormItem,
	FormLabel,
	FormMessage,
} from "../ui/form";
import { Label } from "../ui/label";
import { RadioGroup, RadioGroupItem } from "../ui/radio-group";
import { Textarea } from "../ui/textarea";

export const formSchema = z.object({
	type: z.union([z.literal("bug"), z.literal("feature")]),
	feedback: z.string().min(10),
});

export type FormValues = z.infer<typeof formSchema>;
export type SubmitHandler = (
	values: FormValues,
	form: UseFormReturn<FormValues>,
) => Promise<void>;

const { Form, Submit } = createSchemaForm(formSchema);
export { Form, Submit };

export const Type = () => {
	const { control } = useFormContext<FormValues>();
	return (
		<FormField
			control={control}
			name="type"
			render={({ field }) => (
				<FormItem>
					<FormLabel>Name</FormLabel>
					<FormControl>
						<RadioGroup
							defaultValue="bug"
							onValueChange={field.onChange}
						>
							<div className="flex items-center space-x-2">
								<RadioGroupItem value="bug" id="bug" />
								<Label htmlFor="bug">
									<Icon icon={faBug} className="mr-2" />
									Bug
								</Label>
							</div>
							<div className="flex items-center space-x-2">
								<RadioGroupItem value="feature" id="feature" />
								<Label htmlFor="feature">
									<Icon
										icon={faConciergeBell}
										className="mr-2"
									/>
									Feature
								</Label>
							</div>
						</RadioGroup>
					</FormControl>
					<FormMessage />
				</FormItem>
			)}
		/>
	);
};

export const Feedback = () => {
	const { control } = useFormContext<FormValues>();
	return (
		<FormField
			control={control}
			name="feedback"
			render={({ field }) => (
				<FormItem>
					<FormLabel>Feedback</FormLabel>
					<FormControl>
						<Textarea
							placeholder="Write anything you want, report an issue, request a feature or just say hi!"
							{...field}
						/>
					</FormControl>
					<FormMessage />
				</FormItem>
			)}
		/>
	);
};
