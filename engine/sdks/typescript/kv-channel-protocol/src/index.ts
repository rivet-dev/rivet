// KV Channel Protocol v1 - TypeScript types and BARE serialization
//
// Hand-written from engine/sdks/schemas/kv-channel-protocol/v1.bare.
// Follow the same read/write/encode/decode pattern as
// engine/sdks/typescript/runner-protocol/src/index.ts.

import * as bare from "@rivetkit/bare-ts"

const DEFAULT_CONFIG = /* @__PURE__ */ bare.Config({})

export const PROTOCOL_VERSION = 1;

// MARK: Core

export type i64 = bigint
export type u32 = number

export type Id = string

function readId(bc: bare.ByteCursor): Id {
	return bare.readString(bc)
}

function writeId(bc: bare.ByteCursor, x: Id): void {
	bare.writeString(bc, x)
}

// MARK: Actor Session

export type ActorOpenRequest = null

export type ActorCloseRequest = null

export type ActorOpenResponse = null

export type ActorCloseResponse = null

// MARK: KV

export type KvKey = ArrayBuffer

function readKvKey(bc: bare.ByteCursor): KvKey {
	return bare.readData(bc)
}

function writeKvKey(bc: bare.ByteCursor, x: KvKey): void {
	bare.writeData(bc, x)
}

export type KvValue = ArrayBuffer

function readKvValue(bc: bare.ByteCursor): KvValue {
	return bare.readData(bc)
}

function writeKvValue(bc: bare.ByteCursor, x: KvValue): void {
	bare.writeData(bc, x)
}

export type KvGetRequest = {
	readonly keys: readonly KvKey[]
}

function readKvGetRequest(bc: bare.ByteCursor): KvGetRequest {
	const len = bare.readUintSafe(bc)
	const keys: KvKey[] = []
	for (let i = 0; i < len; i++) {
		keys.push(readKvKey(bc))
	}
	return { keys }
}

function writeKvGetRequest(bc: bare.ByteCursor, x: KvGetRequest): void {
	bare.writeUintSafe(bc, x.keys.length)
	for (let i = 0; i < x.keys.length; i++) {
		writeKvKey(bc, x.keys[i])
	}
}

export type KvPutRequest = {
	readonly keys: readonly KvKey[]
	readonly values: readonly KvValue[]
}

function readKvPutRequest(bc: bare.ByteCursor): KvPutRequest {
	const keysLen = bare.readUintSafe(bc)
	const keys: KvKey[] = []
	for (let i = 0; i < keysLen; i++) {
		keys.push(readKvKey(bc))
	}
	const valuesLen = bare.readUintSafe(bc)
	const values: KvValue[] = []
	for (let i = 0; i < valuesLen; i++) {
		values.push(readKvValue(bc))
	}
	return { keys, values }
}

function writeKvPutRequest(bc: bare.ByteCursor, x: KvPutRequest): void {
	bare.writeUintSafe(bc, x.keys.length)
	for (let i = 0; i < x.keys.length; i++) {
		writeKvKey(bc, x.keys[i])
	}
	bare.writeUintSafe(bc, x.values.length)
	for (let i = 0; i < x.values.length; i++) {
		writeKvValue(bc, x.values[i])
	}
}

export type KvDeleteRequest = {
	readonly keys: readonly KvKey[]
}

function readKvDeleteRequest(bc: bare.ByteCursor): KvDeleteRequest {
	const len = bare.readUintSafe(bc)
	const keys: KvKey[] = []
	for (let i = 0; i < len; i++) {
		keys.push(readKvKey(bc))
	}
	return { keys }
}

function writeKvDeleteRequest(bc: bare.ByteCursor, x: KvDeleteRequest): void {
	bare.writeUintSafe(bc, x.keys.length)
	for (let i = 0; i < x.keys.length; i++) {
		writeKvKey(bc, x.keys[i])
	}
}

export type KvDeleteRangeRequest = {
	readonly start: KvKey
	readonly end: KvKey
}

function readKvDeleteRangeRequest(bc: bare.ByteCursor): KvDeleteRangeRequest {
	return {
		start: readKvKey(bc),
		end: readKvKey(bc),
	}
}

function writeKvDeleteRangeRequest(bc: bare.ByteCursor, x: KvDeleteRangeRequest): void {
	writeKvKey(bc, x.start)
	writeKvKey(bc, x.end)
}

// MARK: Request/Response

export type RequestData =
	| { readonly tag: "ActorOpenRequest"; readonly val: ActorOpenRequest }
	| { readonly tag: "ActorCloseRequest"; readonly val: ActorCloseRequest }
	| { readonly tag: "KvGetRequest"; readonly val: KvGetRequest }
	| { readonly tag: "KvPutRequest"; readonly val: KvPutRequest }
	| { readonly tag: "KvDeleteRequest"; readonly val: KvDeleteRequest }
	| { readonly tag: "KvDeleteRangeRequest"; readonly val: KvDeleteRangeRequest }

function readRequestData(bc: bare.ByteCursor): RequestData {
	const offset = bc.offset
	const tag = bare.readU8(bc)
	switch (tag) {
		case 0:
			return { tag: "ActorOpenRequest", val: null }
		case 1:
			return { tag: "ActorCloseRequest", val: null }
		case 2:
			return { tag: "KvGetRequest", val: readKvGetRequest(bc) }
		case 3:
			return { tag: "KvPutRequest", val: readKvPutRequest(bc) }
		case 4:
			return { tag: "KvDeleteRequest", val: readKvDeleteRequest(bc) }
		case 5:
			return { tag: "KvDeleteRangeRequest", val: readKvDeleteRangeRequest(bc) }
		default: {
			bc.offset = offset
			throw new bare.BareError(offset, "invalid tag")
		}
	}
}

function writeRequestData(bc: bare.ByteCursor, x: RequestData): void {
	switch (x.tag) {
		case "ActorOpenRequest": {
			bare.writeU8(bc, 0)
			break
		}
		case "ActorCloseRequest": {
			bare.writeU8(bc, 1)
			break
		}
		case "KvGetRequest": {
			bare.writeU8(bc, 2)
			writeKvGetRequest(bc, x.val)
			break
		}
		case "KvPutRequest": {
			bare.writeU8(bc, 3)
			writeKvPutRequest(bc, x.val)
			break
		}
		case "KvDeleteRequest": {
			bare.writeU8(bc, 4)
			writeKvDeleteRequest(bc, x.val)
			break
		}
		case "KvDeleteRangeRequest": {
			bare.writeU8(bc, 5)
			writeKvDeleteRangeRequest(bc, x.val)
			break
		}
	}
}

export type ErrorResponse = {
	readonly code: string
	readonly message: string
}

function readErrorResponse(bc: bare.ByteCursor): ErrorResponse {
	return {
		code: bare.readString(bc),
		message: bare.readString(bc),
	}
}

function writeErrorResponse(bc: bare.ByteCursor, x: ErrorResponse): void {
	bare.writeString(bc, x.code)
	bare.writeString(bc, x.message)
}

export type KvGetResponse = {
	readonly keys: readonly KvKey[]
	readonly values: readonly KvValue[]
}

function readKvGetResponse(bc: bare.ByteCursor): KvGetResponse {
	const keysLen = bare.readUintSafe(bc)
	const keys: KvKey[] = []
	for (let i = 0; i < keysLen; i++) {
		keys.push(readKvKey(bc))
	}
	const valuesLen = bare.readUintSafe(bc)
	const values: KvValue[] = []
	for (let i = 0; i < valuesLen; i++) {
		values.push(readKvValue(bc))
	}
	return { keys, values }
}

function writeKvGetResponse(bc: bare.ByteCursor, x: KvGetResponse): void {
	bare.writeUintSafe(bc, x.keys.length)
	for (let i = 0; i < x.keys.length; i++) {
		writeKvKey(bc, x.keys[i])
	}
	bare.writeUintSafe(bc, x.values.length)
	for (let i = 0; i < x.values.length; i++) {
		writeKvValue(bc, x.values[i])
	}
}

export type KvPutResponse = null

export type KvDeleteResponse = null

export type ResponseData =
	| { readonly tag: "ErrorResponse"; readonly val: ErrorResponse }
	| { readonly tag: "ActorOpenResponse"; readonly val: ActorOpenResponse }
	| { readonly tag: "ActorCloseResponse"; readonly val: ActorCloseResponse }
	| { readonly tag: "KvGetResponse"; readonly val: KvGetResponse }
	| { readonly tag: "KvPutResponse"; readonly val: KvPutResponse }
	| { readonly tag: "KvDeleteResponse"; readonly val: KvDeleteResponse }

function readResponseData(bc: bare.ByteCursor): ResponseData {
	const offset = bc.offset
	const tag = bare.readU8(bc)
	switch (tag) {
		case 0:
			return { tag: "ErrorResponse", val: readErrorResponse(bc) }
		case 1:
			return { tag: "ActorOpenResponse", val: null }
		case 2:
			return { tag: "ActorCloseResponse", val: null }
		case 3:
			return { tag: "KvGetResponse", val: readKvGetResponse(bc) }
		case 4:
			return { tag: "KvPutResponse", val: null }
		case 5:
			return { tag: "KvDeleteResponse", val: null }
		default: {
			bc.offset = offset
			throw new bare.BareError(offset, "invalid tag")
		}
	}
}

function writeResponseData(bc: bare.ByteCursor, x: ResponseData): void {
	switch (x.tag) {
		case "ErrorResponse": {
			bare.writeU8(bc, 0)
			writeErrorResponse(bc, x.val)
			break
		}
		case "ActorOpenResponse": {
			bare.writeU8(bc, 1)
			break
		}
		case "ActorCloseResponse": {
			bare.writeU8(bc, 2)
			break
		}
		case "KvGetResponse": {
			bare.writeU8(bc, 3)
			writeKvGetResponse(bc, x.val)
			break
		}
		case "KvPutResponse": {
			bare.writeU8(bc, 4)
			break
		}
		case "KvDeleteResponse": {
			bare.writeU8(bc, 5)
			break
		}
	}
}

// MARK: To Server

export type ToServerRequest = {
	readonly requestId: u32
	readonly actorId: Id
	readonly data: RequestData
}

function readToServerRequest(bc: bare.ByteCursor): ToServerRequest {
	return {
		requestId: bare.readU32(bc),
		actorId: readId(bc),
		data: readRequestData(bc),
	}
}

function writeToServerRequest(bc: bare.ByteCursor, x: ToServerRequest): void {
	bare.writeU32(bc, x.requestId)
	writeId(bc, x.actorId)
	writeRequestData(bc, x.data)
}

export type ToServerPong = {
	readonly ts: i64
}

function readToServerPong(bc: bare.ByteCursor): ToServerPong {
	return {
		ts: bare.readI64(bc),
	}
}

function writeToServerPong(bc: bare.ByteCursor, x: ToServerPong): void {
	bare.writeI64(bc, x.ts)
}

export type ToServer =
	| { readonly tag: "ToServerRequest"; readonly val: ToServerRequest }
	| { readonly tag: "ToServerPong"; readonly val: ToServerPong }

export function readToServer(bc: bare.ByteCursor): ToServer {
	const offset = bc.offset
	const tag = bare.readU8(bc)
	switch (tag) {
		case 0:
			return { tag: "ToServerRequest", val: readToServerRequest(bc) }
		case 1:
			return { tag: "ToServerPong", val: readToServerPong(bc) }
		default: {
			bc.offset = offset
			throw new bare.BareError(offset, "invalid tag")
		}
	}
}

export function writeToServer(bc: bare.ByteCursor, x: ToServer): void {
	switch (x.tag) {
		case "ToServerRequest": {
			bare.writeU8(bc, 0)
			writeToServerRequest(bc, x.val)
			break
		}
		case "ToServerPong": {
			bare.writeU8(bc, 1)
			writeToServerPong(bc, x.val)
			break
		}
	}
}

export function encodeToServer(x: ToServer, config?: Partial<bare.Config>): Uint8Array {
	const fullConfig = config != null ? bare.Config(config) : DEFAULT_CONFIG
	const bc = new bare.ByteCursor(
		new Uint8Array(fullConfig.initialBufferLength),
		fullConfig,
	)
	writeToServer(bc, x)
	return new Uint8Array(bc.view.buffer, bc.view.byteOffset, bc.offset)
}

export function decodeToServer(bytes: Uint8Array): ToServer {
	const bc = new bare.ByteCursor(bytes, DEFAULT_CONFIG)
	const result = readToServer(bc)
	if (bc.offset < bc.view.byteLength) {
		throw new bare.BareError(bc.offset, "remaining bytes")
	}
	return result
}

// MARK: To Client

export type ToClientResponse = {
	readonly requestId: u32
	readonly data: ResponseData
}

function readToClientResponse(bc: bare.ByteCursor): ToClientResponse {
	return {
		requestId: bare.readU32(bc),
		data: readResponseData(bc),
	}
}

function writeToClientResponse(bc: bare.ByteCursor, x: ToClientResponse): void {
	bare.writeU32(bc, x.requestId)
	writeResponseData(bc, x.data)
}

export type ToClientPing = {
	readonly ts: i64
}

function readToClientPing(bc: bare.ByteCursor): ToClientPing {
	return {
		ts: bare.readI64(bc),
	}
}

function writeToClientPing(bc: bare.ByteCursor, x: ToClientPing): void {
	bare.writeI64(bc, x.ts)
}

export type ToClientClose = null

export type ToClient =
	| { readonly tag: "ToClientResponse"; readonly val: ToClientResponse }
	| { readonly tag: "ToClientPing"; readonly val: ToClientPing }
	| { readonly tag: "ToClientClose"; readonly val: ToClientClose }

export function readToClient(bc: bare.ByteCursor): ToClient {
	const offset = bc.offset
	const tag = bare.readU8(bc)
	switch (tag) {
		case 0:
			return { tag: "ToClientResponse", val: readToClientResponse(bc) }
		case 1:
			return { tag: "ToClientPing", val: readToClientPing(bc) }
		case 2:
			return { tag: "ToClientClose", val: null }
		default: {
			bc.offset = offset
			throw new bare.BareError(offset, "invalid tag")
		}
	}
}

export function writeToClient(bc: bare.ByteCursor, x: ToClient): void {
	switch (x.tag) {
		case "ToClientResponse": {
			bare.writeU8(bc, 0)
			writeToClientResponse(bc, x.val)
			break
		}
		case "ToClientPing": {
			bare.writeU8(bc, 1)
			writeToClientPing(bc, x.val)
			break
		}
		case "ToClientClose": {
			bare.writeU8(bc, 2)
			break
		}
	}
}

export function encodeToClient(x: ToClient, config?: Partial<bare.Config>): Uint8Array {
	const fullConfig = config != null ? bare.Config(config) : DEFAULT_CONFIG
	const bc = new bare.ByteCursor(
		new Uint8Array(fullConfig.initialBufferLength),
		fullConfig,
	)
	writeToClient(bc, x)
	return new Uint8Array(bc.view.buffer, bc.view.byteOffset, bc.offset)
}

export function decodeToClient(bytes: Uint8Array): ToClient {
	const bc = new bare.ByteCursor(bytes, DEFAULT_CONFIG)
	const result = readToClient(bc)
	if (bc.offset < bc.view.byteLength) {
		throw new bare.BareError(bc.offset, "remaining bytes")
	}
	return result
}
