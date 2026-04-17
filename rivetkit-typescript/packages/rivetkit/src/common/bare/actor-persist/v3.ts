// Vendored BARE codec. Keep the wire format compatible with the existing runtime.

import * as bare from "@rivetkit/bare-ts"

const config = /* @__PURE__ */ bare.Config({})

export type i64 = bigint
export type u16 = number

export type GatewayId = ArrayBuffer

export function readGatewayId(bc: bare.ByteCursor): GatewayId {
    return bare.readFixedData(bc, 4)
}

export function writeGatewayId(bc: bare.ByteCursor, x: GatewayId): void {
    assert(x.byteLength === 4)
    bare.writeFixedData(bc, x)
}

export type RequestId = ArrayBuffer

export function readRequestId(bc: bare.ByteCursor): RequestId {
    return bare.readFixedData(bc, 4)
}

export function writeRequestId(bc: bare.ByteCursor, x: RequestId): void {
    assert(x.byteLength === 4)
    bare.writeFixedData(bc, x)
}

export type MessageIndex = u16

export function readMessageIndex(bc: bare.ByteCursor): MessageIndex {
    return bare.readU16(bc)
}

export function writeMessageIndex(bc: bare.ByteCursor, x: MessageIndex): void {
    bare.writeU16(bc, x)
}

export function encodeMessageIndex(x: MessageIndex): Uint8Array {
    const bc = new bare.ByteCursor(
        new Uint8Array(config.initialBufferLength),
        config
    )
    writeMessageIndex(bc, x)
    return new Uint8Array(bc.view.buffer, bc.view.byteOffset, bc.offset)
}

export function decodeMessageIndex(bytes: Uint8Array): MessageIndex {
    const bc = new bare.ByteCursor(bytes, config)
    const result = readMessageIndex(bc)
    if (bc.offset < bc.view.byteLength) {
        throw new bare.BareError(bc.offset, "remaining bytes")
    }
    return result
}

export type Cbor = ArrayBuffer

export function readCbor(bc: bare.ByteCursor): Cbor {
    return bare.readData(bc)
}

export function writeCbor(bc: bare.ByteCursor, x: Cbor): void {
    bare.writeData(bc, x)
}

export type Subscription = {
    readonly eventName: string,
}

export function readSubscription(bc: bare.ByteCursor): Subscription {
    return {
        eventName: bare.readString(bc),
    }
}

export function writeSubscription(bc: bare.ByteCursor, x: Subscription): void {
    bare.writeString(bc, x.eventName)
}

function read0(bc: bare.ByteCursor): readonly Subscription[] {
    const len = bare.readUintSafe(bc)
    if (len === 0) { return [] }
    const result = [readSubscription(bc)]
    for (let i = 1; i < len; i++) {
        result[i] = readSubscription(bc)
    }
    return result
}

function write0(bc: bare.ByteCursor, x: readonly Subscription[]): void {
    bare.writeUintSafe(bc, x.length)
    for (let i = 0; i < x.length; i++) {
        writeSubscription(bc, x[i])
    }
}

function read1(bc: bare.ByteCursor): ReadonlyMap<string, string> {
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

function write1(bc: bare.ByteCursor, x: ReadonlyMap<string, string>): void {
    bare.writeUintSafe(bc, x.size)
    for(const kv of x) {
        bare.writeString(bc, kv[0])
        bare.writeString(bc, kv[1])
    }
}

export type Conn = {
    readonly id: string,
    readonly parameters: Cbor,
    readonly state: Cbor,
    readonly subscriptions: readonly Subscription[],
    readonly gatewayId: GatewayId,
    readonly requestId: RequestId,
    readonly serverMessageIndex: u16,
    readonly clientMessageIndex: u16,
    readonly requestPath: string,
    readonly requestHeaders: ReadonlyMap<string, string>,
}

export function readConn(bc: bare.ByteCursor): Conn {
    return {
        id: bare.readString(bc),
        parameters: readCbor(bc),
        state: readCbor(bc),
        subscriptions: read0(bc),
        gatewayId: readGatewayId(bc),
        requestId: readRequestId(bc),
        serverMessageIndex: bare.readU16(bc),
        clientMessageIndex: bare.readU16(bc),
        requestPath: bare.readString(bc),
        requestHeaders: read1(bc),
    }
}

export function writeConn(bc: bare.ByteCursor, x: Conn): void {
    bare.writeString(bc, x.id)
    writeCbor(bc, x.parameters)
    writeCbor(bc, x.state)
    write0(bc, x.subscriptions)
    writeGatewayId(bc, x.gatewayId)
    writeRequestId(bc, x.requestId)
    bare.writeU16(bc, x.serverMessageIndex)
    bare.writeU16(bc, x.clientMessageIndex)
    bare.writeString(bc, x.requestPath)
    write1(bc, x.requestHeaders)
}

export function encodeConn(x: Conn): Uint8Array {
    const bc = new bare.ByteCursor(
        new Uint8Array(config.initialBufferLength),
        config
    )
    writeConn(bc, x)
    return new Uint8Array(bc.view.buffer, bc.view.byteOffset, bc.offset)
}

export function decodeConn(bytes: Uint8Array): Conn {
    const bc = new bare.ByteCursor(bytes, config)
    const result = readConn(bc)
    if (bc.offset < bc.view.byteLength) {
        throw new bare.BareError(bc.offset, "remaining bytes")
    }
    return result
}

function read2(bc: bare.ByteCursor): Cbor | null {
    return bare.readBool(bc)
        ? readCbor(bc)
        : null
}

function write2(bc: bare.ByteCursor, x: Cbor | null): void {
    bare.writeBool(bc, x !== null)
    if (x !== null) {
        writeCbor(bc, x)
    }
}

export type ScheduleEvent = {
    readonly eventId: string,
    readonly timestamp: i64,
    readonly action: string,
    readonly args: Cbor | null,
}

export function readScheduleEvent(bc: bare.ByteCursor): ScheduleEvent {
    return {
        eventId: bare.readString(bc),
        timestamp: bare.readI64(bc),
        action: bare.readString(bc),
        args: read2(bc),
    }
}

export function writeScheduleEvent(bc: bare.ByteCursor, x: ScheduleEvent): void {
    bare.writeString(bc, x.eventId)
    bare.writeI64(bc, x.timestamp)
    bare.writeString(bc, x.action)
    write2(bc, x.args)
}

function read3(bc: bare.ByteCursor): readonly ScheduleEvent[] {
    const len = bare.readUintSafe(bc)
    if (len === 0) { return [] }
    const result = [readScheduleEvent(bc)]
    for (let i = 1; i < len; i++) {
        result[i] = readScheduleEvent(bc)
    }
    return result
}

function write3(bc: bare.ByteCursor, x: readonly ScheduleEvent[]): void {
    bare.writeUintSafe(bc, x.length)
    for (let i = 0; i < x.length; i++) {
        writeScheduleEvent(bc, x[i])
    }
}

export type Actor = {
    readonly input: Cbor | null,
    readonly hasInitialized: boolean,
    readonly state: Cbor,
    readonly scheduledEvents: readonly ScheduleEvent[],
}

export function readActor(bc: bare.ByteCursor): Actor {
    return {
        input: read2(bc),
        hasInitialized: bare.readBool(bc),
        state: readCbor(bc),
        scheduledEvents: read3(bc),
    }
}

export function writeActor(bc: bare.ByteCursor, x: Actor): void {
    write2(bc, x.input)
    bare.writeBool(bc, x.hasInitialized)
    writeCbor(bc, x.state)
    write3(bc, x.scheduledEvents)
}

export function encodeActor(x: Actor): Uint8Array {
    const bc = new bare.ByteCursor(
        new Uint8Array(config.initialBufferLength),
        config
    )
    writeActor(bc, x)
    return new Uint8Array(bc.view.buffer, bc.view.byteOffset, bc.offset)
}

export function decodeActor(bytes: Uint8Array): Actor {
    const bc = new bare.ByteCursor(bytes, config)
    const result = readActor(bc)
    if (bc.offset < bc.view.byteLength) {
        throw new bare.BareError(bc.offset, "remaining bytes")
    }
    return result
}


function assert(condition: boolean, message?: string): asserts condition {
    if (!condition) throw new Error(message ?? "Assertion failed")
}
