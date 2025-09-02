import type { ComponentProps } from "react";
import z from "zod";
import {
	Button,
	createSchemaForm,
	FormField,
	FormItem,
	FormLabel,
	FormMessage,
	Input,
} from "@/components";

const connectionFormSchema = z.object({
	username: z
		.string()
		.url("Please enter a valid URL")
		.min(1, "URL is required"),
	token: z.string().min(1, "Token is required"),
});

const { Form, Submit: ConnectionSubmit } =
	createSchemaForm(connectionFormSchema);

export const ConnectionForm = (
	props: Omit<ComponentProps<typeof Form>, "children">,
) => {
	return (
		<Form {...props}>
			<div className="flex flex-col gap-2">
				<FormField
					name="username"
					render={({ field }) => (
						<FormItem>
							<FormLabel>Endpoint</FormLabel>
							<Input
								type="url"
								placeholder="http://localhost:8080"
								{...field}
							/>
							<FormMessage />
						</FormItem>
					)}
				/>
				<FormField
					name="token"
					render={({ field }) => (
						<FormItem>
							<FormLabel>Token</FormLabel>
							<Input
								type="password"
								placeholder="Enter your access token"
								{...field}
							/>
							<FormMessage />
						</FormItem>
					)}
				/>
				<div className="flex justify-center">
					<ConnectionSubmit asChild allowPristine>
						<Button type="submit" className="mt-4 mx-auto">
							Connect
						</Button>
					</ConnectionSubmit>
				</div>
			</div>
		</Form>
	);
};
