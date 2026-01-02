import type {
	Location,
	LoopIterationMarker,
	NameIndex,
	PathSegment,
	Storage,
} from "./types.js";

/**
 * Check if a path segment is a loop iteration marker.
 */
export function isLoopIterationMarker(
	segment: PathSegment,
): segment is LoopIterationMarker {
	return typeof segment === "object" && "loop" in segment;
}

/**
 * Register a name in the registry and return its index.
 * If the name already exists, returns the existing index.
 */
export function registerName(storage: Storage, name: string): NameIndex {
	const existing = storage.nameRegistry.indexOf(name);
	if (existing !== -1) {
		return existing;
	}
	storage.nameRegistry.push(name);
	return storage.nameRegistry.length - 1;
}

/**
 * Resolve a name index to its string value.
 */
export function resolveName(storage: Storage, index: NameIndex): string {
	const name = storage.nameRegistry[index];
	if (name === undefined) {
		throw new Error(`Name index ${index} not found in registry`);
	}
	return name;
}

/**
 * Convert a location to a KV key string.
 * Named entries use their string name, loop iterations use ~N format.
 */
export function locationToKey(storage: Storage, location: Location): string {
	return location
		.map((segment) => {
			if (typeof segment === "number") {
				return resolveName(storage, segment);
			}
			return `~${segment.iteration}`;
		})
		.join("/");
}

/**
 * Append a named segment to a location.
 */
export function appendName(
	storage: Storage,
	location: Location,
	name: string,
): Location {
	const nameIndex = registerName(storage, name);
	return [...location, nameIndex];
}

/**
 * Append a loop iteration segment to a location.
 */
export function appendLoopIteration(
	storage: Storage,
	location: Location,
	loopName: string,
	iteration: number,
): Location {
	const loopIndex = registerName(storage, loopName);
	return [...location, { loop: loopIndex, iteration }];
}

/**
 * Create an empty location (root).
 */
export function emptyLocation(): Location {
	return [];
}

/**
 * Get the parent location (all segments except the last).
 */
export function parentLocation(location: Location): Location {
	return location.slice(0, -1);
}

/**
 * Check if one location is a prefix of another.
 */
export function isLocationPrefix(
	prefix: Location,
	location: Location,
): boolean {
	if (prefix.length > location.length) {
		return false;
	}
	for (let i = 0; i < prefix.length; i++) {
		const prefixSegment = prefix[i];
		const locationSegment = location[i];

		if (typeof prefixSegment === "number" && typeof locationSegment === "number") {
			if (prefixSegment !== locationSegment) {
				return false;
			}
		} else if (
			isLoopIterationMarker(prefixSegment) &&
			isLoopIterationMarker(locationSegment)
		) {
			if (
				prefixSegment.loop !== locationSegment.loop ||
				prefixSegment.iteration !== locationSegment.iteration
			) {
				return false;
			}
		} else {
			return false;
		}
	}
	return true;
}

/**
 * Compare two locations for equality.
 */
export function locationsEqual(a: Location, b: Location): boolean {
	if (a.length !== b.length) {
		return false;
	}
	return isLocationPrefix(a, b);
}

/**
 * Get all entry keys that are children of a given location.
 *
 * Note: Returns a map of key → entry for convenience, not key → location.
 * The location can be retrieved from the entry itself via entry.location.
 */
export function getChildEntries(
	storage: Storage,
	parentLoc: Location,
): Map<string, Location> {
	const parentKey = locationToKey(storage, parentLoc);
	const children = new Map<string, Location>();

	for (const [key, entry] of storage.history.entries) {
		// Handle empty parent (root) - all entries are children
		const isChild =
			parentKey === ""
				? true
				: key.startsWith(parentKey + "/") || key === parentKey;

		if (isChild) {
			// Return the actual entry's location, not the parent location
			children.set(key, entry.location);
		}
	}

	return children;
}
