// @generated - post-processed by build.rs
export const PROTOCOL_VERSION = 1;

import * as bare from "@rivetkit/bare-ts"

const DEFAULT_CONFIG = /* @__PURE__ */ bare.Config({})

export type i64 = bigint
export type u32 = number

/**
 * Id is a 30-character base36 string encoding the V1 format from
 * engine/packages/util-id/. Use the util-id library for parsing
 * and validation. Do not hand-roll Id parsing.
 */
export type Id = string

export function readId(bc: bare.ByteCursor): Id {
    return bare.readString(bc)
}

export function writeId(bc: bare.ByteCursor, x: Id): void {
    bare.writeString(bc, x)
}

/**
 * actorId is on ToRivetRequest, not on open/close. The outer
 * actorId is the single source of truth for routing.
 */
export type ActorOpenRequest = null

export type ActorCloseRequest = null

export type ActorOpenResponse = null

export type ActorCloseResponse = null

export type KvKey = ArrayBuffer

export function readKvKey(bc: bare.ByteCursor): KvKey {
    return bare.readData(bc)
}

export function writeKvKey(bc: bare.ByteCursor, x: KvKey): void {
    bare.writeData(bc, x)
}

export type KvValue = ArrayBuffer

export function readKvValue(bc: bare.ByteCursor): KvValue {
    return bare.readData(bc)
}

export function writeKvValue(bc: bare.ByteCursor, x: KvValue): void {
    bare.writeData(bc, x)
}

function read0(bc: bare.ByteCursor): readonly KvKey[] {
    const len = bare.readUintSafe(bc)
    if (len === 0) {
        return []
    }
    const result = [readKvKey(bc)]
    for (let i = 1; i < len; i++) {
        result[i] = readKvKey(bc)
    }
    return result
}

function write0(bc: bare.ByteCursor, x: readonly KvKey[]): void {
    bare.writeUintSafe(bc, x.length)
    for (let i = 0; i < x.length; i++) {
        writeKvKey(bc, x[i])
    }
}

export type KvGetRequest = {
    readonly keys: readonly KvKey[]
}

export function readKvGetRequest(bc: bare.ByteCursor): KvGetRequest {
    return {
        keys: read0(bc),
    }
}

export function writeKvGetRequest(bc: bare.ByteCursor, x: KvGetRequest): void {
    write0(bc, x.keys)
}

function read1(bc: bare.ByteCursor): readonly KvValue[] {
    const len = bare.readUintSafe(bc)
    if (len === 0) {
        return []
    }
    const result = [readKvValue(bc)]
    for (let i = 1; i < len; i++) {
        result[i] = readKvValue(bc)
    }
    return result
}

function write1(bc: bare.ByteCursor, x: readonly KvValue[]): void {
    bare.writeUintSafe(bc, x.length)
    for (let i = 0; i < x.length; i++) {
        writeKvValue(bc, x[i])
    }
}

export type KvPutRequest = {
    /**
     * keys and values are parallel lists. keys.len() must equal values.len().
     */
    readonly keys: readonly KvKey[]
    readonly values: readonly KvValue[]
}

export function readKvPutRequest(bc: bare.ByteCursor): KvPutRequest {
    return {
        keys: read0(bc),
        values: read1(bc),
    }
}

export function writeKvPutRequest(bc: bare.ByteCursor, x: KvPutRequest): void {
    write0(bc, x.keys)
    write1(bc, x.values)
}

export type KvDeleteRequest = {
    readonly keys: readonly KvKey[]
}

export function readKvDeleteRequest(bc: bare.ByteCursor): KvDeleteRequest {
    return {
        keys: read0(bc),
    }
}

export function writeKvDeleteRequest(bc: bare.ByteCursor, x: KvDeleteRequest): void {
    write0(bc, x.keys)
}

export type KvDeleteRangeRequest = {
    readonly start: KvKey
    readonly end: KvKey
}

export function readKvDeleteRangeRequest(bc: bare.ByteCursor): KvDeleteRangeRequest {
    return {
        start: readKvKey(bc),
        end: readKvKey(bc),
    }
}

export function writeKvDeleteRangeRequest(bc: bare.ByteCursor, x: KvDeleteRangeRequest): void {
    writeKvKey(bc, x.start)
    writeKvKey(bc, x.end)
}

export type RequestData =
    | { readonly tag: "ActorOpenRequest"; readonly val: ActorOpenRequest }
    | { readonly tag: "ActorCloseRequest"; readonly val: ActorCloseRequest }
    | { readonly tag: "KvGetRequest"; readonly val: KvGetRequest }
    | { readonly tag: "KvPutRequest"; readonly val: KvPutRequest }
    | { readonly tag: "KvDeleteRequest"; readonly val: KvDeleteRequest }
    | { readonly tag: "KvDeleteRangeRequest"; readonly val: KvDeleteRangeRequest }

export function readRequestData(bc: bare.ByteCursor): RequestData {
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

export function writeRequestData(bc: bare.ByteCursor, x: RequestData): void {
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

export function readErrorResponse(bc: bare.ByteCursor): ErrorResponse {
    return {
        code: bare.readString(bc),
        message: bare.readString(bc),
    }
}

export function writeErrorResponse(bc: bare.ByteCursor, x: ErrorResponse): void {
    bare.writeString(bc, x.code)
    bare.writeString(bc, x.message)
}

export type KvGetResponse = {
    /**
     * Only keys that exist are returned. Missing keys are omitted.
     * The client infers missing keys by comparing request keys to
     * response keys. This matches the runner protocol behavior
     * (engine/packages/pegboard/src/actor_kv/mod.rs).
     */
    readonly keys: readonly KvKey[]
    readonly values: readonly KvValue[]
}

export function readKvGetResponse(bc: bare.ByteCursor): KvGetResponse {
    return {
        keys: read0(bc),
        values: read1(bc),
    }
}

export function writeKvGetResponse(bc: bare.ByteCursor, x: KvGetResponse): void {
    write0(bc, x.keys)
    write1(bc, x.values)
}

export type KvPutResponse = null

/**
 * KvDeleteResponse is used for both KvDeleteRequest and
 * KvDeleteRangeRequest, same as the runner protocol.
 */
export type KvDeleteResponse = null

export type ResponseData =
    | { readonly tag: "ErrorResponse"; readonly val: ErrorResponse }
    | { readonly tag: "ActorOpenResponse"; readonly val: ActorOpenResponse }
    | { readonly tag: "ActorCloseResponse"; readonly val: ActorCloseResponse }
    | { readonly tag: "KvGetResponse"; readonly val: KvGetResponse }
    | { readonly tag: "KvPutResponse"; readonly val: KvPutResponse }
    | { readonly tag: "KvDeleteResponse"; readonly val: KvDeleteResponse }

export function readResponseData(bc: bare.ByteCursor): ResponseData {
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

export function writeResponseData(bc: bare.ByteCursor, x: ResponseData): void {
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

export type ToRivetRequest = {
    readonly requestId: u32
    readonly actorId: Id
    readonly data: RequestData
}

export function readToRivetRequest(bc: bare.ByteCursor): ToRivetRequest {
    return {
        requestId: bare.readU32(bc),
        actorId: readId(bc),
        data: readRequestData(bc),
    }
}

export function writeToRivetRequest(bc: bare.ByteCursor, x: ToRivetRequest): void {
    bare.writeU32(bc, x.requestId)
    writeId(bc, x.actorId)
    writeRequestData(bc, x.data)
}

export type ToRivetPong = {
    readonly ts: i64
}

export function readToRivetPong(bc: bare.ByteCursor): ToRivetPong {
    return {
        ts: bare.readI64(bc),
    }
}

export function writeToRivetPong(bc: bare.ByteCursor, x: ToRivetPong): void {
    bare.writeI64(bc, x.ts)
}

export type ToRivet =
    | { readonly tag: "ToRivetRequest"; readonly val: ToRivetRequest }
    | { readonly tag: "ToRivetPong"; readonly val: ToRivetPong }

export function readToRivet(bc: bare.ByteCursor): ToRivet {
    const offset = bc.offset
    const tag = bare.readU8(bc)
    switch (tag) {
        case 0:
            return { tag: "ToRivetRequest", val: readToRivetRequest(bc) }
        case 1:
            return { tag: "ToRivetPong", val: readToRivetPong(bc) }
        default: {
            bc.offset = offset
            throw new bare.BareError(offset, "invalid tag")
        }
    }
}

export function writeToRivet(bc: bare.ByteCursor, x: ToRivet): void {
    switch (x.tag) {
        case "ToRivetRequest": {
            bare.writeU8(bc, 0)
            writeToRivetRequest(bc, x.val)
            break
        }
        case "ToRivetPong": {
            bare.writeU8(bc, 1)
            writeToRivetPong(bc, x.val)
            break
        }
    }
}

export function encodeToRivet(x: ToRivet, config?: Partial<bare.Config>): Uint8Array {
    const fullConfig = config != null ? bare.Config(config) : DEFAULT_CONFIG
    const bc = new bare.ByteCursor(
        new Uint8Array(fullConfig.initialBufferLength),
        fullConfig,
    )
    writeToRivet(bc, x)
    return new Uint8Array(bc.view.buffer, bc.view.byteOffset, bc.offset)
}

export function decodeToRivet(bytes: Uint8Array): ToRivet {
    const bc = new bare.ByteCursor(bytes, DEFAULT_CONFIG)
    const result = readToRivet(bc)
    if (bc.offset < bc.view.byteLength) {
        throw new bare.BareError(bc.offset, "remaining bytes")
    }
    return result
}

export type ToClientResponse = {
    readonly requestId: u32
    readonly data: ResponseData
}

export function readToClientResponse(bc: bare.ByteCursor): ToClientResponse {
    return {
        requestId: bare.readU32(bc),
        data: readResponseData(bc),
    }
}

export function writeToClientResponse(bc: bare.ByteCursor, x: ToClientResponse): void {
    bare.writeU32(bc, x.requestId)
    writeResponseData(bc, x.data)
}

export type ToClientPing = {
    readonly ts: i64
}

export function readToClientPing(bc: bare.ByteCursor): ToClientPing {
    return {
        ts: bare.readI64(bc),
    }
}

export function writeToClientPing(bc: bare.ByteCursor, x: ToClientPing): void {
    bare.writeI64(bc, x.ts)
}

/**
 * Server-initiated close. Sent when the server is shutting down
 * or draining connections. The client should close all actors
 * and reconnect with backoff. Same pattern as the runner
 * protocol's ToRunnerClose.
 */
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


function assert(condition: boolean, message?: string): asserts condition {
    if (!condition) throw new Error(message ?? "Assertion failed")
}
