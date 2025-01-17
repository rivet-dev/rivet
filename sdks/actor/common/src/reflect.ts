export function listObjectMethods(obj: object | null): (string | symbol)[] {
	if (obj === null || obj === Object.prototype) {
		return [];
	}

	return [
		...Reflect.ownKeys(obj),
		...listObjectMethods(Reflect.getPrototypeOf(obj)),
	];
}
