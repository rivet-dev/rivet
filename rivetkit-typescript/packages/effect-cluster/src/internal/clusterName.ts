/**
 * Encodings of cluster addresses as RivetKit actor keys and as opaque names.
 *
 * @internal
 */
/**
 * Encodes `(type, id)` as one string, length-prefixed so the pair stays
 * collision-free without restricting the characters of either part.
 */
export const encodeName = (type: string, id: string): string =>
	`${type.length}:${type}${id}`;

const lengthPrefix = /^([1-9]\d*):/;

/** Decodes a name produced by {@link encodeName}, or `undefined` for any other string. */
export const decodeName = (
	name: string,
): { readonly type: string; readonly id: string } | undefined => {
	const match = lengthPrefix.exec(name);
	if (match === null) return undefined;
	const typeLength = Number(match[1]);
	const payload = name.slice(match[0].length);
	if (typeLength > payload.length) return undefined;
	return {
		type: payload.slice(0, typeLength),
		id: payload.slice(typeLength),
	};
};
