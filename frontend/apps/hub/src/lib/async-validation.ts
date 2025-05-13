import type { Rivet } from "@rivet-gg/api-full";
import { toast } from "@rivet-gg/components";
import type { FieldValues, Path, UseFormReturn } from "react-hook-form";
import z from "zod";
import {
	TraversableErrors,
	VALIDATION_ERRORS,
	type ValidationPaths,
} from "./traversable-errors";
import { isRivetError } from "./utils";

export function validateAgainstApi<TGroup extends keyof ValidationPaths>({
	group,
	errors,
}: {
	group: TGroup;
	errors: Rivet.ValidationError[];
}) {
	const traversable = new TraversableErrors(VALIDATION_ERRORS[group]);
	traversable.load(errors.map((e) => e.path));

	return {
		setFormErrors: <TValues extends FieldValues>(
			form: UseFormReturn<TValues>,
			fields: Record<Path<TValues>, keyof ValidationPaths[TGroup]>,
		) => {
			if (traversable.isEmpty())
				return { isValid: traversable.isEmpty() };

			for (const [field, mappedField] of Object.entries(fields)) {
				const fieldErrors = traversable.findFormatted(
					mappedField as string,
				);
				if (fieldErrors.length > 0) {
					form.setError(field as Path<TValues>, {
						type: "manual",
						message: fieldErrors[0] || "",
					});
				}
			}

			return { isValid: traversable.isEmpty() };
		},
		setSchemaIssues: <TValues extends FieldValues>(
			ctx: z.RefinementCtx,
			fields: Record<Path<TValues>, keyof ValidationPaths[TGroup]>,
		) => {
			if (traversable.isEmpty())
				return { isValid: traversable.isEmpty() };

			for (const [field, mappedField] of Object.entries(fields)) {
				const fieldErrors = traversable.findFormatted(
					mappedField as string,
				);
				if (fieldErrors.length > 0) {
					ctx.addIssue({
						path: field.split("."),
						code: z.ZodIssueCode.custom,
						message: fieldErrors[0] || "",
					});
				}
			}

			return { isValid: traversable.isEmpty() };
		},
	};
}

export async function safeAsyncValidation(
	ctx: z.RefinementCtx,
	fn: () => Promise<void>,
	opts: { message?: string } = {},
) {
	try {
		await fn();
	} catch (e) {
		const msg = opts.message || "An error occurred while validating.";

		toast.error(msg, {
			description: isRivetError(e) ? e.body.message : undefined,
		});
		ctx.addIssue({
			path: [""],
			code: z.ZodIssueCode.custom,
		});
	}
}
