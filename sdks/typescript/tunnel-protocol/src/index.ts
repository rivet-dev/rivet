import assert from "node:assert"
import * as bare from "@bare-ts/lib"

const DEFAULT_CONFIG = /* @__PURE__ */ bare.Config({})

export type u16 = number

export type RequestId = ArrayBuffer

export function readRequestId(bc: bare.ByteCursor): RequestId {
    return bare.readFixedData(bc, 16)
}

export function writeRequestId(bc: bare.ByteCursor, x: RequestId): void {
    assert(x.byteLength === 16)
    bare.writeFixedData(bc, x)
}

/**
 * UUIDv4
 */
export type MessageId = ArrayBuffer

export function readMessageId(bc: bare.ByteCursor): MessageId {
    return bare.readFixedData(bc, 16)
}

export function writeMessageId(bc: bare.ByteCursor, x: MessageId): void {
    assert(x.byteLength === 16)
    bare.writeFixedData(bc, x)
}

/**
 * UUIDv4
 */
export type Id = string

export function readId(bc: bare.ByteCursor): Id {
    return bare.readString(bc)
}

export function writeId(bc: bare.ByteCursor, x: Id): void {
    bare.writeString(bc, x)
}

/**
 * MARK: Ack
 */
export type Ack = null

function read0(bc: bare.ByteCursor): ReadonlyMap<string, string> {
    const len = bare.readUintSafe(bc)
    const result = new Map<string, string>()
    for (let i = 0; i < len; i++) {
        const offset = bc.offset
        const key = bare.readString(bc)
        if (result.has(key)) {
            bc.offset = offset
            throw new bare.BareError(offset, "duplicated key")
        }
        result.set(key, bare.readString(bc))
    }
    return result
}

function write0(bc: bare.ByteCursor, x: ReadonlyMap<string, string>): void {
    bare.writeUintSafe(bc, x.size)
    for (const kv of x) {
        bare.writeString(bc, kv[0])
        bare.writeString(bc, kv[1])
    }
}

function read1(bc: bare.ByteCursor): ArrayBuffer | null {
    return bare.readBool(bc) ? bare.readData(bc) : null
}

function write1(bc: bare.ByteCursor, x: ArrayBuffer | null): void {
    bare.writeBool(bc, x != null)
    if (x != null) {
        bare.writeData(bc, x)
    }
}

/**
 * MARK: HTTP
 */
export type ToServerRequestStart = {
    readonly actorId: Id
    readonly method: string
    readonly path: string
    readonly headers: ReadonlyMap<string, string>
    readonly body: ArrayBuffer | null
    readonly stream: boolean
}

export function readToServerRequestStart(bc: bare.ByteCursor): ToServerRequestStart {
    return {
        actorId: readId(bc),
        method: bare.readString(bc),
        path: bare.readString(bc),
        headers: read0(bc),
        body: read1(bc),
        stream: bare.readBool(bc),
    }
}

export function writeToServerRequestStart(bc: bare.ByteCursor, x: ToServerRequestStart): void {
    writeId(bc, x.actorId)
    bare.writeString(bc, x.method)
    bare.writeString(bc, x.path)
    write0(bc, x.headers)
    write1(bc, x.body)
    bare.writeBool(bc, x.stream)
}

export type ToServerRequestChunk = {
    readonly body: ArrayBuffer
    readonly finish: boolean
}

export function readToServerRequestChunk(bc: bare.ByteCursor): ToServerRequestChunk {
    return {
        body: bare.readData(bc),
        finish: bare.readBool(bc),
    }
}

export function writeToServerRequestChunk(bc: bare.ByteCursor, x: ToServerRequestChunk): void {
    bare.writeData(bc, x.body)
    bare.writeBool(bc, x.finish)
}

export type ToServerRequestAbort = null

export type ToClientResponseStart = {
    readonly status: u16
    readonly headers: ReadonlyMap<string, string>
    readonly body: ArrayBuffer | null
    readonly stream: boolean
}

export function readToClientResponseStart(bc: bare.ByteCursor): ToClientResponseStart {
    return {
        status: bare.readU16(bc),
        headers: read0(bc),
        body: read1(bc),
        stream: bare.readBool(bc),
    }
}

export function writeToClientResponseStart(bc: bare.ByteCursor, x: ToClientResponseStart): void {
    bare.writeU16(bc, x.status)
    write0(bc, x.headers)
    write1(bc, x.body)
    bare.writeBool(bc, x.stream)
}

export type ToClientResponseChunk = {
    readonly body: ArrayBuffer
    readonly finish: boolean
}

export function readToClientResponseChunk(bc: bare.ByteCursor): ToClientResponseChunk {
    return {
        body: bare.readData(bc),
        finish: bare.readBool(bc),
    }
}

export function writeToClientResponseChunk(bc: bare.ByteCursor, x: ToClientResponseChunk): void {
    bare.writeData(bc, x.body)
    bare.writeBool(bc, x.finish)
}

export type ToClientResponseAbort = null

/**
 * MARK: WebSocket
 */
export type ToServerWebSocketOpen = {
    readonly actorId: Id
    readonly path: string
    readonly headers: ReadonlyMap<string, string>
}

export function readToServerWebSocketOpen(bc: bare.ByteCursor): ToServerWebSocketOpen {
    return {
        actorId: readId(bc),
        path: bare.readString(bc),
        headers: read0(bc),
    }
}

export function writeToServerWebSocketOpen(bc: bare.ByteCursor, x: ToServerWebSocketOpen): void {
    writeId(bc, x.actorId)
    bare.writeString(bc, x.path)
    write0(bc, x.headers)
}

export type ToServerWebSocketMessage = {
    readonly data: ArrayBuffer
    readonly binary: boolean
}

export function readToServerWebSocketMessage(bc: bare.ByteCursor): ToServerWebSocketMessage {
    return {
        data: bare.readData(bc),
        binary: bare.readBool(bc),
    }
}

export function writeToServerWebSocketMessage(bc: bare.ByteCursor, x: ToServerWebSocketMessage): void {
    bare.writeData(bc, x.data)
    bare.writeBool(bc, x.binary)
}

function read2(bc: bare.ByteCursor): u16 | null {
    return bare.readBool(bc) ? bare.readU16(bc) : null
}

function write2(bc: bare.ByteCursor, x: u16 | null): void {
    bare.writeBool(bc, x != null)
    if (x != null) {
        bare.writeU16(bc, x)
    }
}

function read3(bc: bare.ByteCursor): string | null {
    return bare.readBool(bc) ? bare.readString(bc) : null
}

function write3(bc: bare.ByteCursor, x: string | null): void {
    bare.writeBool(bc, x != null)
    if (x != null) {
        bare.writeString(bc, x)
    }
}

export type ToServerWebSocketClose = {
    readonly code: u16 | null
    readonly reason: string | null
}

export function readToServerWebSocketClose(bc: bare.ByteCursor): ToServerWebSocketClose {
    return {
        code: read2(bc),
        reason: read3(bc),
    }
}

export function writeToServerWebSocketClose(bc: bare.ByteCursor, x: ToServerWebSocketClose): void {
    write2(bc, x.code)
    write3(bc, x.reason)
}

export type ToClientWebSocketOpen = null

export type ToClientWebSocketMessage = {
    readonly data: ArrayBuffer
    readonly binary: boolean
}

export function readToClientWebSocketMessage(bc: bare.ByteCursor): ToClientWebSocketMessage {
    return {
        data: bare.readData(bc),
        binary: bare.readBool(bc),
    }
}

export function writeToClientWebSocketMessage(bc: bare.ByteCursor, x: ToClientWebSocketMessage): void {
    bare.writeData(bc, x.data)
    bare.writeBool(bc, x.binary)
}

export type ToClientWebSocketClose = {
    readonly code: u16 | null
    readonly reason: string | null
}

export function readToClientWebSocketClose(bc: bare.ByteCursor): ToClientWebSocketClose {
    return {
        code: read2(bc),
        reason: read3(bc),
    }
}

export function writeToClientWebSocketClose(bc: bare.ByteCursor, x: ToClientWebSocketClose): void {
    write2(bc, x.code)
    write3(bc, x.reason)
}

/**
 * MARK: Message
 */
export type MessageKind =
    | { readonly tag: "Ack"; readonly val: Ack }
    /**
     * HTTP
     */
    | { readonly tag: "ToServerRequestStart"; readonly val: ToServerRequestStart }
    | { readonly tag: "ToServerRequestChunk"; readonly val: ToServerRequestChunk }
    | { readonly tag: "ToServerRequestAbort"; readonly val: ToServerRequestAbort }
    | { readonly tag: "ToClientResponseStart"; readonly val: ToClientResponseStart }
    | { readonly tag: "ToClientResponseChunk"; readonly val: ToClientResponseChunk }
    | { readonly tag: "ToClientResponseAbort"; readonly val: ToClientResponseAbort }
    /**
     * WebSocket
     */
    | { readonly tag: "ToServerWebSocketOpen"; readonly val: ToServerWebSocketOpen }
    | { readonly tag: "ToServerWebSocketMessage"; readonly val: ToServerWebSocketMessage }
    | { readonly tag: "ToServerWebSocketClose"; readonly val: ToServerWebSocketClose }
    | { readonly tag: "ToClientWebSocketOpen"; readonly val: ToClientWebSocketOpen }
    | { readonly tag: "ToClientWebSocketMessage"; readonly val: ToClientWebSocketMessage }
    | { readonly tag: "ToClientWebSocketClose"; readonly val: ToClientWebSocketClose }

export function readMessageKind(bc: bare.ByteCursor): MessageKind {
    const offset = bc.offset
    const tag = bare.readU8(bc)
    switch (tag) {
        case 0:
            return { tag: "Ack", val: null }
        case 1:
            return { tag: "ToServerRequestStart", val: readToServerRequestStart(bc) }
        case 2:
            return { tag: "ToServerRequestChunk", val: readToServerRequestChunk(bc) }
        case 3:
            return { tag: "ToServerRequestAbort", val: null }
        case 4:
            return { tag: "ToClientResponseStart", val: readToClientResponseStart(bc) }
        case 5:
            return { tag: "ToClientResponseChunk", val: readToClientResponseChunk(bc) }
        case 6:
            return { tag: "ToClientResponseAbort", val: null }
        case 7:
            return { tag: "ToServerWebSocketOpen", val: readToServerWebSocketOpen(bc) }
        case 8:
            return { tag: "ToServerWebSocketMessage", val: readToServerWebSocketMessage(bc) }
        case 9:
            return { tag: "ToServerWebSocketClose", val: readToServerWebSocketClose(bc) }
        case 10:
            return { tag: "ToClientWebSocketOpen", val: null }
        case 11:
            return { tag: "ToClientWebSocketMessage", val: readToClientWebSocketMessage(bc) }
        case 12:
            return { tag: "ToClientWebSocketClose", val: readToClientWebSocketClose(bc) }
        default: {
            bc.offset = offset
            throw new bare.BareError(offset, "invalid tag")
        }
    }
}

export function writeMessageKind(bc: bare.ByteCursor, x: MessageKind): void {
    switch (x.tag) {
        case "Ack": {
            bare.writeU8(bc, 0)
            break
        }
        case "ToServerRequestStart": {
            bare.writeU8(bc, 1)
            writeToServerRequestStart(bc, x.val)
            break
        }
        case "ToServerRequestChunk": {
            bare.writeU8(bc, 2)
            writeToServerRequestChunk(bc, x.val)
            break
        }
        case "ToServerRequestAbort": {
            bare.writeU8(bc, 3)
            break
        }
        case "ToClientResponseStart": {
            bare.writeU8(bc, 4)
            writeToClientResponseStart(bc, x.val)
            break
        }
        case "ToClientResponseChunk": {
            bare.writeU8(bc, 5)
            writeToClientResponseChunk(bc, x.val)
            break
        }
        case "ToClientResponseAbort": {
            bare.writeU8(bc, 6)
            break
        }
        case "ToServerWebSocketOpen": {
            bare.writeU8(bc, 7)
            writeToServerWebSocketOpen(bc, x.val)
            break
        }
        case "ToServerWebSocketMessage": {
            bare.writeU8(bc, 8)
            writeToServerWebSocketMessage(bc, x.val)
            break
        }
        case "ToServerWebSocketClose": {
            bare.writeU8(bc, 9)
            writeToServerWebSocketClose(bc, x.val)
            break
        }
        case "ToClientWebSocketOpen": {
            bare.writeU8(bc, 10)
            break
        }
        case "ToClientWebSocketMessage": {
            bare.writeU8(bc, 11)
            writeToClientWebSocketMessage(bc, x.val)
            break
        }
        case "ToClientWebSocketClose": {
            bare.writeU8(bc, 12)
            writeToClientWebSocketClose(bc, x.val)
            break
        }
    }
}

/**
 * MARK: Message sent over tunnel WebSocket
 */
export type RunnerMessage = {
    readonly requestId: RequestId
    readonly messageId: MessageId
    readonly messageKind: MessageKind
}

export function readRunnerMessage(bc: bare.ByteCursor): RunnerMessage {
    return {
        requestId: readRequestId(bc),
        messageId: readMessageId(bc),
        messageKind: readMessageKind(bc),
    }
}

export function writeRunnerMessage(bc: bare.ByteCursor, x: RunnerMessage): void {
    writeRequestId(bc, x.requestId)
    writeMessageId(bc, x.messageId)
    writeMessageKind(bc, x.messageKind)
}

export function encodeRunnerMessage(x: RunnerMessage, config?: Partial<bare.Config>): Uint8Array {
    const fullConfig = config != null ? bare.Config(config) : DEFAULT_CONFIG
    const bc = new bare.ByteCursor(
        new Uint8Array(fullConfig.initialBufferLength),
        fullConfig,
    )
    writeRunnerMessage(bc, x)
    return new Uint8Array(bc.view.buffer, bc.view.byteOffset, bc.offset)
}

export function decodeRunnerMessage(bytes: Uint8Array): RunnerMessage {
    const bc = new bare.ByteCursor(bytes, DEFAULT_CONFIG)
    const result = readRunnerMessage(bc)
    if (bc.offset < bc.view.byteLength) {
        throw new bare.BareError(bc.offset, "remaining bytes")
    }
    return result
}

/**
 * MARK: Message sent over UPS
 */
export type PubSubMessage = {
    readonly requestId: RequestId
    readonly messageId: MessageId
    /**
     * Subject to send replies to. Only sent when opening a new request from gateway -> runner.
     */
    readonly replyTo: string | null
    readonly messageKind: MessageKind
}

export function readPubSubMessage(bc: bare.ByteCursor): PubSubMessage {
    return {
        requestId: readRequestId(bc),
        messageId: readMessageId(bc),
        replyTo: read3(bc),
        messageKind: readMessageKind(bc),
    }
}

export function writePubSubMessage(bc: bare.ByteCursor, x: PubSubMessage): void {
    writeRequestId(bc, x.requestId)
    writeMessageId(bc, x.messageId)
    write3(bc, x.replyTo)
    writeMessageKind(bc, x.messageKind)
}

export function encodePubSubMessage(x: PubSubMessage, config?: Partial<bare.Config>): Uint8Array {
    const fullConfig = config != null ? bare.Config(config) : DEFAULT_CONFIG
    const bc = new bare.ByteCursor(
        new Uint8Array(fullConfig.initialBufferLength),
        fullConfig,
    )
    writePubSubMessage(bc, x)
    return new Uint8Array(bc.view.buffer, bc.view.byteOffset, bc.offset)
}

export function decodePubSubMessage(bytes: Uint8Array): PubSubMessage {
    const bc = new bare.ByteCursor(bytes, DEFAULT_CONFIG)
    const result = readPubSubMessage(bc)
    if (bc.offset < bc.view.byteLength) {
        throw new bare.BareError(bc.offset, "remaining bytes")
    }
    return result
}
