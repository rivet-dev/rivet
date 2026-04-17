// Vendored BARE codec. Keep the wire format compatible with the existing runtime.

import * as bare from "@rivetkit/bare-ts"

const config = /* @__PURE__ */ bare.Config({})

export type i64 = bigint
export type u16 = number
export type u32 = number
export type u64 = bigint

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

export type QueueMetadata = {
    readonly nextId: u64,
    readonly size: u32,
}

export function readQueueMetadata(bc: bare.ByteCursor): QueueMetadata {
    return {
        nextId: bare.readU64(bc),
        size: bare.readU32(bc),
    }
}

export function writeQueueMetadata(bc: bare.ByteCursor, x: QueueMetadata): void {
    bare.writeU64(bc, x.nextId)
    bare.writeU32(bc, x.size)
}

export function encodeQueueMetadata(x: QueueMetadata): Uint8Array {
    const bc = new bare.ByteCursor(
        new Uint8Array(config.initialBufferLength),
        config
    )
    writeQueueMetadata(bc, x)
    return new Uint8Array(bc.view.buffer, bc.view.byteOffset, bc.offset)
}

export function decodeQueueMetadata(bytes: Uint8Array): QueueMetadata {
    const bc = new bare.ByteCursor(bytes, config)
    const result = readQueueMetadata(bc)
    if (bc.offset < bc.view.byteLength) {
        throw new bare.BareError(bc.offset, "remaining bytes")
    }
    return result
}

function read4(bc: bare.ByteCursor): u32 | null {
    return bare.readBool(bc)
        ? bare.readU32(bc)
        : null
}

function write4(bc: bare.ByteCursor, x: u32 | null): void {
    bare.writeBool(bc, x !== null)
    if (x !== null) {
        bare.writeU32(bc, x)
    }
}

function read5(bc: bare.ByteCursor): i64 | null {
    return bare.readBool(bc)
        ? bare.readI64(bc)
        : null
}

function write5(bc: bare.ByteCursor, x: i64 | null): void {
    bare.writeBool(bc, x !== null)
    if (x !== null) {
        bare.writeI64(bc, x)
    }
}

function read6(bc: bare.ByteCursor): boolean | null {
    return bare.readBool(bc)
        ? bare.readBool(bc)
        : null
}

function write6(bc: bare.ByteCursor, x: boolean | null): void {
    bare.writeBool(bc, x !== null)
    if (x !== null) {
        bare.writeBool(bc, x)
    }
}

export type QueueMessage = {
    readonly name: string,
    readonly body: Cbor,
    readonly createdAt: i64,
    readonly failureCount: u32 | null,
    readonly availableAt: i64 | null,
    readonly inFlight: boolean | null,
    readonly inFlightAt: i64 | null,
}

export function readQueueMessage(bc: bare.ByteCursor): QueueMessage {
    return {
        name: bare.readString(bc),
        body: readCbor(bc),
        createdAt: bare.readI64(bc),
        failureCount: read4(bc),
        availableAt: read5(bc),
        inFlight: read6(bc),
        inFlightAt: read5(bc),
    }
}

export function writeQueueMessage(bc: bare.ByteCursor, x: QueueMessage): void {
    bare.writeString(bc, x.name)
    writeCbor(bc, x.body)
    bare.writeI64(bc, x.createdAt)
    write4(bc, x.failureCount)
    write5(bc, x.availableAt)
    write6(bc, x.inFlight)
    write5(bc, x.inFlightAt)
}

export function encodeQueueMessage(x: QueueMessage): Uint8Array {
    const bc = new bare.ByteCursor(
        new Uint8Array(config.initialBufferLength),
        config
    )
    writeQueueMessage(bc, x)
    return new Uint8Array(bc.view.buffer, bc.view.byteOffset, bc.offset)
}

export function decodeQueueMessage(bytes: Uint8Array): QueueMessage {
    const bc = new bare.ByteCursor(bytes, config)
    const result = readQueueMessage(bc)
    if (bc.offset < bc.view.byteLength) {
        throw new bare.BareError(bc.offset, "remaining bytes")
    }
    return result
}


function assert(condition: boolean, message?: string): asserts condition {
    if (!condition) throw new Error(message ?? "Assertion failed")
}
