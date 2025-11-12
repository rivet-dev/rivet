import type { ComponentProps } from "react";
import z from "zod";
import {
	Accordion,
	AccordionContent,
	AccordionItem,
	AccordionTrigger,
	Button,
	createSchemaForm,
	FormDescription,
	FormField,
	FormItem,
	FormLabel,
	FormMessage,
	Input,
} from "@/components";

const connectionFormSchema = z.object({
	url: z.string().url("Please enter a valid URL").min(1, "URL is required"),
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
					name="url"
					render={({ field }) => (
						<FormItem>
							<FormLabel>Endpoint</FormLabel>
							<Input
								type="url"
								placeholder="http://localhost:6420"
								{...field}
							/>
							<FormMessage />
						</FormItem>
					)}
				/>
				{/* <Accordion type="single" collapsible className="-mx-1">
					<AccordionItem value="token">
						<AccordionTrigger className="px-1">
							Advanced
						</AccordionTrigger>

						<AccordionContent className="mt-2 px-1">
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
										<FormDescription>
											Connecting to Rivet Engine? You will
											need to provide an access token
											here.
										</FormDescription>
										<FormMessage />
									</FormItem>
								)}
							/>
						</AccordionContent>
					</AccordionItem>
				</Accordion> */}
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
