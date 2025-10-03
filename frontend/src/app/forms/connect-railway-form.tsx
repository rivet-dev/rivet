import type { UseFormReturn } from "react-hook-form";
import z from "zod";
import * as ConnectVercelForm from "@/app/forms/connect-vercel-form";
import { createSchemaForm } from "@/components";

export const formSchema = z.object({
	endpoint: z.string().url(),
});

export type FormValues = z.infer<typeof formSchema>;
export type SubmitHandler = (
	values: FormValues,
	form: UseFormReturn<FormValues>,
) => Promise<void>;

const { Form, Submit, SetValue } = createSchemaForm(formSchema);
export { Form, Submit, SetValue };

export const ConnectionCheck = ConnectVercelForm.ConnectionCheck;
export const Endpoint = ConnectVercelForm.Endpoint;
