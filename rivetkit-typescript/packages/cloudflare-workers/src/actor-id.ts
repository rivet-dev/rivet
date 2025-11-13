/**
 * Actor ID utilities for managing actor IDs with generation tracking.
 *
 * Actor IDs are formatted as: `{doId}:{generation}`
 * This allows tracking actor resurrection and preventing stale references.
 */

/**
 * Build an actor ID from a Durable Object ID and generation number.
 * @param doId The Durable Object ID
 * @param generation The generation number (increments on resurrection)
 * @returns The formatted actor ID
 */
export function buildActorId(doId: string, generation: number): string {
	return `${doId}:${generation}`;
}

/**
 * Parse an actor ID into its components.
 * @param actorId The actor ID to parse
 * @returns A tuple of [doId, generation]
 * @throws Error if the actor ID format is invalid
 */
export function parseActorId(actorId: string): [string, number] {
	const parts = actorId.split(":");
	if (parts.length !== 2) {
		throw new Error(`Invalid actor ID format: ${actorId}`);
	}

	const [doId, generationStr] = parts;
	const generation = parseInt(generationStr, 10);

	if (Number.isNaN(generation)) {
		throw new Error(`Invalid generation number in actor ID: ${actorId}`);
	}

	return [doId, generation];
}
