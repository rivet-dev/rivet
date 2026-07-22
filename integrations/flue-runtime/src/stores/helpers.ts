import type {
	DirectAgentSubmissionInput,
	DispatchAgentSubmissionInput,
	DispatchInput,
	SessionEntry,
} from '@flue/runtime/adapter-kit';
import { SUBMISSION_HARNESS_NAME, SUBMISSION_SESSION_NAME } from './constants.js';

export interface PersistedImageChunk {
	imageId: string;
	index: number;
	count: number;
	data: string;
}

export interface PersistedChunkOwner {
	kind: 'session_entry' | 'submission';
	id: string;
	part: string;
}

export interface PersistedChunkRow {
	imageId: string;
	index: number;
	count: number;
	data: string;
}

export function createDispatchAgentSubmissionInput(
	input: DispatchInput,
): DispatchAgentSubmissionInput {
	return { ...input, kind: 'dispatch', submissionId: input.dispatchId };
}

export function createSessionStorageKey(
	instanceId: string,
	harness: string,
	session: string,
): string {
	return `agent-session:${JSON.stringify([instanceId, harness, session])}`;
}

export function createSubmissionSessionKey(instanceId: string): string {
	return createSessionStorageKey(instanceId, SUBMISSION_HARNESS_NAME, SUBMISSION_SESSION_NAME);
}

export function parseAcceptedAt(value: string, label: string): number {
	const acceptedAt = Date.parse(value);
	if (!Number.isFinite(acceptedAt)) {
		throw new Error(`[flue] Internal ${label} received an invalid acceptedAt timestamp.`);
	}
	return acceptedAt;
}

export function sessionEntryChunkOwner(sessionId: string, entryId: string): PersistedChunkOwner {
	return { kind: 'session_entry', id: sessionId, part: entryId };
}

export function submissionChunkOwner(submissionId: string): PersistedChunkOwner {
	return { kind: 'submission', id: submissionId, part: '' };
}

export function prepareSessionEntry(entry: SessionEntry): {
	value: SessionEntry;
	chunks: PersistedImageChunk[];
} {
	if (!isMessageEntryWithImageContent(entry)) return { value: entry, chunks: [] };
	const extracted = extractContentImages(entry.message.content);
	return {
		value: {
			...entry,
			message: { ...entry.message, content: extracted.value },
		} as SessionEntry,
		chunks: extracted.chunks,
	};
}

export function hydratePersistedSessionEntry(
	entry: SessionEntry,
	rows: readonly PersistedChunkRow[],
): SessionEntry {
	if (!isMessageEntryWithImageContent(entry)) {
		assertExactImageGroups([], reassemblePersistedChunks(rows));
		return entry;
	}
	const imageData = reassemblePersistedChunks(rows);
	assertExactImageGroups(markerIds(entry.message.content), imageData);
	return {
		...entry,
		message: { ...entry.message, content: hydrateContentImages(entry.message.content, imageData) },
	} as SessionEntry;
}

export function prepareDirectSubmission(input: DirectAgentSubmissionInput): {
	value: DirectAgentSubmissionInput;
	chunks: PersistedImageChunk[];
} {
	const extracted = extractImageBlocks(input.payload.images as unknown[] | undefined);
	return {
		value: {
			...input,
			payload: {
				...input.payload,
				...(extracted.value === undefined
					? {}
					: { images: extracted.value as DirectAgentSubmissionInput['payload']['images'] }),
			},
		},
		chunks: extracted.chunks,
	};
}

export function hydratePersistedDirectSubmission(
	input: DirectAgentSubmissionInput,
	rows: readonly PersistedChunkRow[],
): DirectAgentSubmissionInput {
	if (input.payload.images === undefined) {
		assertExactImageGroups([], reassemblePersistedChunks(rows));
		return input;
	}
	const imageData = reassemblePersistedChunks(rows);
	assertExactImageGroups(markerIds(input.payload.images), imageData);
	return {
		...input,
		payload: { ...input.payload, images: hydrateImageBlocks(input.payload.images, imageData) },
	};
}

export function matchesPersistedDirectSubmission(
	input: DirectAgentSubmissionInput,
	persistedInput: DirectAgentSubmissionInput,
	rows: readonly PersistedChunkRow[],
): boolean {
	try {
		return (
			JSON.stringify(hydratePersistedDirectSubmission(persistedInput, rows)) ===
			JSON.stringify(input)
		);
	} catch {
		return false;
	}
}

export function samePersistedChunks(
	left: readonly PersistedChunkRow[],
	right: readonly PersistedImageChunk[],
): boolean {
	if (left.length !== right.length) return false;
	const rightByKey = new Map(right.map((chunk) => [chunkKey(chunk), chunk]));
	return left.every((chunk) => {
		const other = rightByKey.get(chunkKey(chunk));
		return other !== undefined && chunk.count === other.count && chunk.data === other.data;
	});
}

export function isSubmissionPayload(
	input: unknown,
	ctx: { kind: string; submissionId: string; sessionKey: string; acceptedAt: number },
): input is DispatchAgentSubmissionInput | DirectAgentSubmissionInput {
	if (!input || typeof input !== 'object') return false;
	const value = input as Record<string, unknown>;
	if (value.kind !== ctx.kind || value.submissionId !== ctx.submissionId) return false;
	if (value.kind === 'dispatch') {
		return (
			typeof value.dispatchId === 'string' &&
			value.dispatchId === value.submissionId &&
			typeof value.agent === 'string' &&
			typeof value.id === 'string' &&
			createSubmissionSessionKey(value.id) === ctx.sessionKey &&
			typeof value.acceptedAt === 'string' &&
			Date.parse(value.acceptedAt) === ctx.acceptedAt &&
			'input' in value &&
			value.input !== undefined
		);
	}
	return (
		typeof value.agent === 'string' &&
		typeof value.id === 'string' &&
		createSubmissionSessionKey(value.id) === ctx.sessionKey &&
		typeof value.acceptedAt === 'string' &&
		Date.parse(value.acceptedAt) === ctx.acceptedAt &&
		isDirectPayload(value.payload)
	);
}

export function clampLimit(
	limit: number | undefined,
	defaultLimit: number,
	maxLimit: number,
): number {
	if (!limit || !Number.isFinite(limit) || limit <= 0) return defaultLimit;
	return Math.min(limit, maxLimit);
}

function isDirectPayload(value: unknown): boolean {
	if (!value || typeof value !== 'object') return false;
	const payload = value as Record<string, unknown>;
	if (typeof payload.message !== 'string') return false;
	if (payload.images === undefined) return true;
	return Array.isArray(payload.images) && payload.images.every(isImageBlock);
}

const IMAGE_DATA_CHUNK_LENGTH = 256 * 1024;
const markerPrefix = '__flue_image_chunks__:';

function extractContentImages(content: unknown): {
	value: unknown;
	chunks: PersistedImageChunk[];
} {
	if (!Array.isArray(content)) return { value: content, chunks: [] };
	return extractImageBlocks(content);
}

function hydrateContentImages(content: unknown, imageData: ReadonlyMap<string, string>): unknown {
	if (!Array.isArray(content)) return content;
	return hydrateImageBlocks(content, imageData);
}

function isMessageEntryWithImageContent(
	entry: SessionEntry,
): entry is SessionEntry & { message: { content: unknown } } {
	if (entry.type !== 'message') return false;
	const message = (entry as { message?: unknown }).message;
	if (!message || typeof message !== 'object') return false;
	const role = (message as { role?: unknown }).role;
	return (
		(role === 'user' || role === 'toolResult') &&
		'content' in (message as Record<string, unknown>)
	);
}

function extractImageBlocks<T>(blocks: T[] | undefined): {
	value: T[] | undefined;
	chunks: PersistedImageChunk[];
} {
	if (blocks === undefined) return { value: undefined, chunks: [] };
	const chunks: PersistedImageChunk[] = [];
	let imageIndex = 0;
	const value = blocks.map((block) => {
		if (!isImageBlock(block)) return block;
		const count = Math.max(1, Math.ceil(block.data.length / IMAGE_DATA_CHUNK_LENGTH));
		const imageId = String(imageIndex++);
		for (let index = 0; index < count; index++) {
			chunks.push({
				imageId,
				index,
				count,
				data: block.data.slice(
					index * IMAGE_DATA_CHUNK_LENGTH,
					(index + 1) * IMAGE_DATA_CHUNK_LENGTH,
				),
			});
		}
		return { ...block, data: `${markerPrefix}${imageId}` };
	}) as T[];
	return { value, chunks };
}

function hydrateImageBlocks<T>(blocks: T[] | undefined, imageData: ReadonlyMap<string, string>): T[] {
	return (blocks ?? []).map((block) => {
		if (!isImageBlock(block) || !block.data.startsWith(markerPrefix)) return block;
		const data = imageData.get(block.data.slice(markerPrefix.length));
		if (data === undefined) throw new Error('[flue] Persisted image chunks are missing.');
		return { ...block, data };
	}) as T[];
}

function markerIds(content: unknown): string[] {
	if (!Array.isArray(content)) return [];
	return content.flatMap((block) => {
		if (!isImageBlock(block) || !block.data.startsWith(markerPrefix)) return [];
		return [block.data.slice(markerPrefix.length)];
	});
}

function reassemblePersistedChunks(
	rows: readonly PersistedChunkRow[],
): ReadonlyMap<string, string> {
	const grouped = new Map<string, PersistedChunkRow[]>();
	for (const row of rows) {
		const imageRows = grouped.get(row.imageId) ?? [];
		imageRows.push(row);
		grouped.set(row.imageId, imageRows);
	}
	const data = new Map<string, string>();
	for (const [imageId, imageRows] of grouped) {
		const ordered = [...imageRows].sort((left, right) => left.index - right.index);
		const expectedCount = ordered[0]?.count;
		if (
			expectedCount === undefined ||
			expectedCount < 1 ||
			ordered.length !== expectedCount ||
			ordered.some((row, index) => row.count !== expectedCount || row.index !== index)
		) {
			throw new Error('[flue] Persisted image chunks are missing or malformed.');
		}
		data.set(imageId, ordered.map((row) => row.data).join(''));
	}
	return data;
}

function assertExactImageGroups(
	markerImageIds: string[],
	imageData: ReadonlyMap<string, string>,
): void {
	const markers = new Set(markerImageIds);
	if (markers.size !== markerImageIds.length || markers.size !== imageData.size) {
		throw new Error('[flue] Persisted image chunks do not match persisted image markers.');
	}
	for (const imageId of imageData.keys()) {
		if (!markers.has(imageId)) {
			throw new Error('[flue] Persisted image chunks do not match persisted image markers.');
		}
	}
}

function isImageBlock(value: unknown): value is {
	type: 'image';
	data: string;
	mimeType?: string;
} {
	if (!value || typeof value !== 'object' || Array.isArray(value)) return false;
	const block = value as { type?: unknown; data?: unknown };
	return block.type === 'image' && typeof block.data === 'string';
}

function chunkKey(chunk: Pick<PersistedChunkRow, 'imageId' | 'index'>): string {
	return `${chunk.imageId}\u0000${chunk.index}`;
}
