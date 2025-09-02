import * as bare from "@bare-ts/lib"

const DEFAULT_CONFIG = /* @__PURE__ */ bare.Config({})

export type u16 = number
export type u64 = bigint

export type RequestId = u64

export function readRequestId(bc: bare.ByteCursor): RequestId {
    return bare.readU64(bc)
}

export function writeRequestId(bc: bare.ByteCursor, x: RequestId): void {
    bare.writeU64(bc, x)
}

export type WebSocketId = u64

export function readWebSocketId(bc: bare.ByteCursor): WebSocketId {
    return bare.readU64(bc)
}

export function writeWebSocketId(bc: bare.ByteCursor, x: WebSocketId): void {
    bare.writeU64(bc, x)
}

export type Id = string

export function readId(bc: bare.ByteCursor): Id {
    return bare.readString(bc)
}

export function writeId(bc: bare.ByteCursor, x: Id): void {
    bare.writeString(bc, x)
}

export enum StreamFinishReason {
    Complete = "Complete",
    Abort = "Abort",
}

export function readStreamFinishReason(bc: bare.ByteCursor): StreamFinishReason {
    const offset = bc.offset
    const tag = bare.readU8(bc)
    switch (tag) {
        case 0:
            return StreamFinishReason.Complete
        case 1:
            return StreamFinishReason.Abort
        default: {
            bc.offset = offset
            throw new bare.BareError(offset, "invalid tag")
        }
    }
}

export function writeStreamFinishReason(bc: bare.ByteCursor, x: StreamFinishReason): void {
    switch (x) {
        case StreamFinishReason.Complete: {
            bare.writeU8(bc, 0)
            break
        }
        case StreamFinishReason.Abort: {
            bare.writeU8(bc, 1)
            break
        }
    }
}

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
 * MARK: HTTP Request Forwarding
 */
export type ToServerRequestStart = {
    readonly requestId: RequestId
    readonly actorId: Id
    readonly method: string
    readonly path: string
    readonly headers: ReadonlyMap<string, string>
    readonly body: ArrayBuffer | null
    readonly stream: boolean
}

export function readToServerRequestStart(bc: bare.ByteCursor): ToServerRequestStart {
    return {
        requestId: readRequestId(bc),
        actorId: readId(bc),
        method: bare.readString(bc),
        path: bare.readString(bc),
        headers: read0(bc),
        body: read1(bc),
        stream: bare.readBool(bc),
    }
}

export function writeToServerRequestStart(bc: bare.ByteCursor, x: ToServerRequestStart): void {
    writeRequestId(bc, x.requestId)
    writeId(bc, x.actorId)
    bare.writeString(bc, x.method)
    bare.writeString(bc, x.path)
    write0(bc, x.headers)
    write1(bc, x.body)
    bare.writeBool(bc, x.stream)
}

export type ToServerRequestChunk = {
    readonly requestId: RequestId
    readonly body: ArrayBuffer
}

export function readToServerRequestChunk(bc: bare.ByteCursor): ToServerRequestChunk {
    return {
        requestId: readRequestId(bc),
        body: bare.readData(bc),
    }
}

export function writeToServerRequestChunk(bc: bare.ByteCursor, x: ToServerRequestChunk): void {
    writeRequestId(bc, x.requestId)
    bare.writeData(bc, x.body)
}

export type ToServerRequestFinish = {
    readonly requestId: RequestId
    readonly reason: StreamFinishReason
}

export function readToServerRequestFinish(bc: bare.ByteCursor): ToServerRequestFinish {
    return {
        requestId: readRequestId(bc),
        reason: readStreamFinishReason(bc),
    }
}

export function writeToServerRequestFinish(bc: bare.ByteCursor, x: ToServerRequestFinish): void {
    writeRequestId(bc, x.requestId)
    writeStreamFinishReason(bc, x.reason)
}

export type ToClientResponseStart = {
    readonly requestId: RequestId
    readonly status: u16
    readonly headers: ReadonlyMap<string, string>
    readonly body: ArrayBuffer | null
    readonly stream: boolean
}

export function readToClientResponseStart(bc: bare.ByteCursor): ToClientResponseStart {
    return {
        requestId: readRequestId(bc),
        status: bare.readU16(bc),
        headers: read0(bc),
        body: read1(bc),
        stream: bare.readBool(bc),
    }
}

export function writeToClientResponseStart(bc: bare.ByteCursor, x: ToClientResponseStart): void {
    writeRequestId(bc, x.requestId)
    bare.writeU16(bc, x.status)
    write0(bc, x.headers)
    write1(bc, x.body)
    bare.writeBool(bc, x.stream)
}

export type ToClientResponseChunk = {
    readonly requestId: RequestId
    readonly body: ArrayBuffer
}

export function readToClientResponseChunk(bc: bare.ByteCursor): ToClientResponseChunk {
    return {
        requestId: readRequestId(bc),
        body: bare.readData(bc),
    }
}

export function writeToClientResponseChunk(bc: bare.ByteCursor, x: ToClientResponseChunk): void {
    writeRequestId(bc, x.requestId)
    bare.writeData(bc, x.body)
}

export type ToClientResponseFinish = {
    readonly requestId: RequestId
    readonly reason: StreamFinishReason
}

export function readToClientResponseFinish(bc: bare.ByteCursor): ToClientResponseFinish {
    return {
        requestId: readRequestId(bc),
        reason: readStreamFinishReason(bc),
    }
}

export function writeToClientResponseFinish(bc: bare.ByteCursor, x: ToClientResponseFinish): void {
    writeRequestId(bc, x.requestId)
    writeStreamFinishReason(bc, x.reason)
}

/**
 * MARK: WebSocket Forwarding
 */
export type ToServerWebSocketOpen = {
    readonly actorId: Id
    readonly webSocketId: WebSocketId
    readonly path: string
    readonly headers: ReadonlyMap<string, string>
}

export function readToServerWebSocketOpen(bc: bare.ByteCursor): ToServerWebSocketOpen {
    return {
        actorId: readId(bc),
        webSocketId: readWebSocketId(bc),
        path: bare.readString(bc),
        headers: read0(bc),
    }
}

export function writeToServerWebSocketOpen(bc: bare.ByteCursor, x: ToServerWebSocketOpen): void {
    writeId(bc, x.actorId)
    writeWebSocketId(bc, x.webSocketId)
    bare.writeString(bc, x.path)
    write0(bc, x.headers)
}

export type ToServerWebSocketMessage = {
    readonly webSocketId: WebSocketId
    readonly data: ArrayBuffer
    readonly binary: boolean
}

export function readToServerWebSocketMessage(bc: bare.ByteCursor): ToServerWebSocketMessage {
    return {
        webSocketId: readWebSocketId(bc),
        data: bare.readData(bc),
        binary: bare.readBool(bc),
    }
}

export function writeToServerWebSocketMessage(bc: bare.ByteCursor, x: ToServerWebSocketMessage): void {
    writeWebSocketId(bc, x.webSocketId)
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
    readonly webSocketId: WebSocketId
    readonly code: u16 | null
    readonly reason: string | null
}

export function readToServerWebSocketClose(bc: bare.ByteCursor): ToServerWebSocketClose {
    return {
        webSocketId: readWebSocketId(bc),
        code: read2(bc),
        reason: read3(bc),
    }
}

export function writeToServerWebSocketClose(bc: bare.ByteCursor, x: ToServerWebSocketClose): void {
    writeWebSocketId(bc, x.webSocketId)
    write2(bc, x.code)
    write3(bc, x.reason)
}

export type ToClientWebSocketOpen = {
    readonly webSocketId: WebSocketId
}

export function readToClientWebSocketOpen(bc: bare.ByteCursor): ToClientWebSocketOpen {
    return {
        webSocketId: readWebSocketId(bc),
    }
}

export function writeToClientWebSocketOpen(bc: bare.ByteCursor, x: ToClientWebSocketOpen): void {
    writeWebSocketId(bc, x.webSocketId)
}

export type ToClientWebSocketMessage = {
    readonly webSocketId: WebSocketId
    readonly data: ArrayBuffer
    readonly binary: boolean
}

export function readToClientWebSocketMessage(bc: bare.ByteCursor): ToClientWebSocketMessage {
    return {
        webSocketId: readWebSocketId(bc),
        data: bare.readData(bc),
        binary: bare.readBool(bc),
    }
}

export function writeToClientWebSocketMessage(bc: bare.ByteCursor, x: ToClientWebSocketMessage): void {
    writeWebSocketId(bc, x.webSocketId)
    bare.writeData(bc, x.data)
    bare.writeBool(bc, x.binary)
}

export type ToClientWebSocketClose = {
    readonly webSocketId: WebSocketId
    readonly code: u16 | null
    readonly reason: string | null
}

export function readToClientWebSocketClose(bc: bare.ByteCursor): ToClientWebSocketClose {
    return {
        webSocketId: readWebSocketId(bc),
        code: read2(bc),
        reason: read3(bc),
    }
}

export function writeToClientWebSocketClose(bc: bare.ByteCursor, x: ToClientWebSocketClose): void {
    writeWebSocketId(bc, x.webSocketId)
    write2(bc, x.code)
    write3(bc, x.reason)
}

/**
 * MARK: Message
 */
export type MessageBody =
    /**
     * HTTP
     */
    | { readonly tag: "ToServerRequestStart"; readonly val: ToServerRequestStart }
    | { readonly tag: "ToServerRequestChunk"; readonly val: ToServerRequestChunk }
    | { readonly tag: "ToServerRequestFinish"; readonly val: ToServerRequestFinish }
    | { readonly tag: "ToClientResponseStart"; readonly val: ToClientResponseStart }
    | { readonly tag: "ToClientResponseChunk"; readonly val: ToClientResponseChunk }
    | { readonly tag: "ToClientResponseFinish"; readonly val: ToClientResponseFinish }
    /**
     * WebSocket
     */
    | { readonly tag: "ToServerWebSocketOpen"; readonly val: ToServerWebSocketOpen }
    | { readonly tag: "ToServerWebSocketMessage"; readonly val: ToServerWebSocketMessage }
    | { readonly tag: "ToServerWebSocketClose"; readonly val: ToServerWebSocketClose }
    | { readonly tag: "ToClientWebSocketOpen"; readonly val: ToClientWebSocketOpen }
    | { readonly tag: "ToClientWebSocketMessage"; readonly val: ToClientWebSocketMessage }
    | { readonly tag: "ToClientWebSocketClose"; readonly val: ToClientWebSocketClose }

export function readMessageBody(bc: bare.ByteCursor): MessageBody {
    const offset = bc.offset
    const tag = bare.readU8(bc)
    switch (tag) {
        case 0:
            return { tag: "ToServerRequestStart", val: readToServerRequestStart(bc) }
        case 1:
            return { tag: "ToServerRequestChunk", val: readToServerRequestChunk(bc) }
        case 2:
            return { tag: "ToServerRequestFinish", val: readToServerRequestFinish(bc) }
        case 3:
            return { tag: "ToClientResponseStart", val: readToClientResponseStart(bc) }
        case 4:
            return { tag: "ToClientResponseChunk", val: readToClientResponseChunk(bc) }
        case 5:
            return { tag: "ToClientResponseFinish", val: readToClientResponseFinish(bc) }
        case 6:
            return { tag: "ToServerWebSocketOpen", val: readToServerWebSocketOpen(bc) }
        case 7:
            return { tag: "ToServerWebSocketMessage", val: readToServerWebSocketMessage(bc) }
        case 8:
            return { tag: "ToServerWebSocketClose", val: readToServerWebSocketClose(bc) }
        case 9:
            return { tag: "ToClientWebSocketOpen", val: readToClientWebSocketOpen(bc) }
        case 10:
            return { tag: "ToClientWebSocketMessage", val: readToClientWebSocketMessage(bc) }
        case 11:
            return { tag: "ToClientWebSocketClose", val: readToClientWebSocketClose(bc) }
        default: {
            bc.offset = offset
            throw new bare.BareError(offset, "invalid tag")
        }
    }
}

export function writeMessageBody(bc: bare.ByteCursor, x: MessageBody): void {
    switch (x.tag) {
        case "ToServerRequestStart": {
            bare.writeU8(bc, 0)
            writeToServerRequestStart(bc, x.val)
            break
        }
        case "ToServerRequestChunk": {
            bare.writeU8(bc, 1)
            writeToServerRequestChunk(bc, x.val)
            break
        }
        case "ToServerRequestFinish": {
            bare.writeU8(bc, 2)
            writeToServerRequestFinish(bc, x.val)
            break
        }
        case "ToClientResponseStart": {
            bare.writeU8(bc, 3)
            writeToClientResponseStart(bc, x.val)
            break
        }
        case "ToClientResponseChunk": {
            bare.writeU8(bc, 4)
            writeToClientResponseChunk(bc, x.val)
            break
        }
        case "ToClientResponseFinish": {
            bare.writeU8(bc, 5)
            writeToClientResponseFinish(bc, x.val)
            break
        }
        case "ToServerWebSocketOpen": {
            bare.writeU8(bc, 6)
            writeToServerWebSocketOpen(bc, x.val)
            break
        }
        case "ToServerWebSocketMessage": {
            bare.writeU8(bc, 7)
            writeToServerWebSocketMessage(bc, x.val)
            break
        }
        case "ToServerWebSocketClose": {
            bare.writeU8(bc, 8)
            writeToServerWebSocketClose(bc, x.val)
            break
        }
        case "ToClientWebSocketOpen": {
            bare.writeU8(bc, 9)
            writeToClientWebSocketOpen(bc, x.val)
            break
        }
        case "ToClientWebSocketMessage": {
            bare.writeU8(bc, 10)
            writeToClientWebSocketMessage(bc, x.val)
            break
        }
        case "ToClientWebSocketClose": {
            bare.writeU8(bc, 11)
            writeToClientWebSocketClose(bc, x.val)
            break
        }
    }
}

/**
 * Main tunnel message
 */
export type TunnelMessage = {
    readonly body: MessageBody
}

export function readTunnelMessage(bc: bare.ByteCursor): TunnelMessage {
    return {
        body: readMessageBody(bc),
    }
}

export function writeTunnelMessage(bc: bare.ByteCursor, x: TunnelMessage): void {
    writeMessageBody(bc, x.body)
}

export function encodeTunnelMessage(x: TunnelMessage, config?: Partial<bare.Config>): Uint8Array {
    const fullConfig = config != null ? bare.Config(config) : DEFAULT_CONFIG
    const bc = new bare.ByteCursor(
        new Uint8Array(fullConfig.initialBufferLength),
        fullConfig,
    )
    writeTunnelMessage(bc, x)
    return new Uint8Array(bc.view.buffer, bc.view.byteOffset, bc.offset)
}

export function decodeTunnelMessage(bytes: Uint8Array): TunnelMessage {
    const bc = new bare.ByteCursor(bytes, DEFAULT_CONFIG)
    const result = readTunnelMessage(bc)
    if (bc.offset < bc.view.byteLength) {
        throw new bare.BareError(bc.offset, "remaining bytes")
    }
    return result
}
