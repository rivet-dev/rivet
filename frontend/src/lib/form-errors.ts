import {
	type FieldPath,
	type FieldValues,
	get,
	type UseFormReturn,
} from "react-hook-form";
import { getApiFieldErrors, toastApiError } from "@/lib/errors";

/**
 * Maps API field paths (or their top-level key) to form fields. Unmapped paths
 * fall back to their top-level key when the form has a value there, so
 * `cidr_allowlist.0` lands on `cidr_allowlist`. Pass the request's keys as
 * `ApiKey` so a renamed API field fails to compile.
 */
export type ApiFieldMap<
	T extends FieldValues,
	ApiKey extends string = string,
> = Partial<Record<ApiKey, FieldPath<T>>>;

function resolveField<T extends FieldValues>(
	form: UseFormReturn<T>,
	field: string,
	fields: ApiFieldMap<T>,
): FieldPath<T> | undefined {
	const topLevel = field.split(".")[0];
	const mapped = fields[field] ?? fields[topLevel];
	if (mapped) return mapped;
	return get(form.getValues(), topLevel) === undefined
		? undefined
		: (topLevel as FieldPath<T>);
}

/**
 * Shows an API error on the form: field errors go to their fields, and
 * anything that matches no field is shown as a toast.
 */
export function showApiErrorOnForm<T extends FieldValues>(
	form: UseFormReturn<T>,
	error: unknown,
	fields: ApiFieldMap<T, string> = {},
) {
	const fieldErrors = getApiFieldErrors(error);
	if (fieldErrors.length === 0) {
		toastApiError(error);
		return;
	}

	const unmatched: string[] = [];
	let focused = false;
	for (const { field, message } of fieldErrors) {
		const name = field ? resolveField(form, field, fields) : undefined;
		if (!name) {
			unmatched.push(field ? `${field}: ${message}` : message);
			continue;
		}
		form.setError(name, { message }, { shouldFocus: !focused });
		focused = true;
	}
	if (unmatched.length > 0) {
		toastApiError(error, unmatched.join("\n"));
	}
}
