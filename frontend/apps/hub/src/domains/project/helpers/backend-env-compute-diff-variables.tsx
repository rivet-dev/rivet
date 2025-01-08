import type { Rivet as RivetEe } from "@rivet-gg/api-ee";

type NewVariable = { key: string; value: string; isSecret: boolean };

export function computeBackendEnvVariablesDiff(
	existingVariables: Record<string, RivetEe.ee.backend.Variable>,
	newVariables: NewVariable[],
) {
	const update: NewVariable[] = [];
	const create: NewVariable[] = [];
	const errors: { idx: number; error: string }[] = [];
	const remove: string[] = [];

	const existingVariableKeys = Object.keys(existingVariables);

	for (const [idx, variable] of newVariables.entries()) {
		if (existingVariableKeys.includes(variable.key)) {
			// Update the variable
			if (variable.value.length > 1) {
				update.push(variable);
			}
		} else {
			if (variable.key.length < 1) {
				errors.push({
					idx,
					error: "Key must be at least 1 character long",
				});
			} else {
				// Create variable
				create.push(variable);
			}
		}
	}

	for (const existingVariableKey of existingVariableKeys) {
		if (!newVariables.find((u) => u.key === existingVariableKey)) {
			// Remove the user
			remove.push(existingVariableKey);
		}
	}

	const finalVariables: Record<string, RivetEe.ee.backend.UpdateVariable> =
		{};
	for (const variable of [...create, ...update]) {
		finalVariables[variable.key] = variable.isSecret
			? { secret: variable.value }
			: { text: variable.value };
	}
	for (const variable of remove) {
		finalVariables[variable] = { delete: true };
	}

	return { variables: finalVariables, errors };
}
