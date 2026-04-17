// Vendored BARE codec. Keep the wire format compatible with the existing runtime.
import * as bare from "@rivetkit/bare-ts"

const config = /* @__PURE__ */ bare.Config({})

export type i64 = bigint

export type PersistedSubscription = {
    readonly eventName: string,
}

export function readPersistedSubscription(bc: bare.ByteCursor): PersistedSubscription {
    return {
        eventName: bare.readString(bc),
    }
}

export function writePersistedSubscription(bc: bare.ByteCursor, x: PersistedSubscription): void {
    bare.writeString(bc, x.eventName)
}

function read0(bc: bare.ByteCursor): readonly PersistedSubscription[] {
    const len = bare.readUintSafe(bc)
    if (len === 0) { return [] }
    const result = [readPersistedSubscription(bc)]
    for (let i = 1; i < len; i++) {
        result[i] = readPersistedSubscription(bc)
    }
    return result
}

function write0(bc: bare.ByteCursor, x: readonly PersistedSubscription[]): void {
    bare.writeUintSafe(bc, x.length)
    for (let i = 0; i < x.length; i++) {
        writePersistedSubscription(bc, x[i])
    }
}

function read1(bc: bare.ByteCursor): ArrayBuffer | null {
    return bare.readBool(bc)
        ? bare.readData(bc)
        : null
}

function write1(bc: bare.ByteCursor, x: ArrayBuffer | null): void {
    bare.writeBool(bc, x !== null)
    if (x !== null) {
        bare.writeData(bc, x)
    }
}

export type PersistedConnection = {
    readonly id: string,
    readonly token: string,
    readonly parameters: ArrayBuffer,
    readonly state: ArrayBuffer,
    readonly subscriptions: readonly PersistedSubscription[],
    readonly lastSeen: i64,
    readonly hibernatableRequestId: ArrayBuffer | null,
}

export function readPersistedConnection(bc: bare.ByteCursor): PersistedConnection {
    return {
        id: bare.readString(bc),
        token: bare.readString(bc),
        parameters: bare.readData(bc),
        state: bare.readData(bc),
        subscriptions: read0(bc),
        lastSeen: bare.readI64(bc),
        hibernatableRequestId: read1(bc),
    }
}

export function writePersistedConnection(bc: bare.ByteCursor, x: PersistedConnection): void {
    bare.writeString(bc, x.id)
    bare.writeString(bc, x.token)
    bare.writeData(bc, x.parameters)
    bare.writeData(bc, x.state)
    write0(bc, x.subscriptions)
    bare.writeI64(bc, x.lastSeen)
    write1(bc, x.hibernatableRequestId)
}

export type GenericPersistedScheduleEvent = {
    readonly action: string,
    readonly args: ArrayBuffer | null,
}

export function readGenericPersistedScheduleEvent(bc: bare.ByteCursor): GenericPersistedScheduleEvent {
    return {
        action: bare.readString(bc),
        args: read1(bc),
    }
}

export function writeGenericPersistedScheduleEvent(bc: bare.ByteCursor, x: GenericPersistedScheduleEvent): void {
    bare.writeString(bc, x.action)
    write1(bc, x.args)
}

export type PersistedScheduleEventKind =
    | { readonly tag: "GenericPersistedScheduleEvent", readonly val: GenericPersistedScheduleEvent }

export function readPersistedScheduleEventKind(bc: bare.ByteCursor): PersistedScheduleEventKind {
    const offset = bc.offset
    const tag = bare.readU8(bc)
    switch (tag) {
        case 0:
            return { tag: "GenericPersistedScheduleEvent", val: readGenericPersistedScheduleEvent(bc) }
        default: {
            bc.offset = offset
            throw new bare.BareError(offset, "invalid tag")
        }
    }
}

export function writePersistedScheduleEventKind(bc: bare.ByteCursor, x: PersistedScheduleEventKind): void {
    switch (x.tag) {
        case "GenericPersistedScheduleEvent": {
            bare.writeU8(bc, 0)
            writeGenericPersistedScheduleEvent(bc, x.val)
            break
        }
    }
}

export type PersistedScheduleEvent = {
    readonly eventId: string,
    readonly timestamp: i64,
    readonly kind: PersistedScheduleEventKind,
}

export function readPersistedScheduleEvent(bc: bare.ByteCursor): PersistedScheduleEvent {
    return {
        eventId: bare.readString(bc),
        timestamp: bare.readI64(bc),
        kind: readPersistedScheduleEventKind(bc),
    }
}

export function writePersistedScheduleEvent(bc: bare.ByteCursor, x: PersistedScheduleEvent): void {
    bare.writeString(bc, x.eventId)
    bare.writeI64(bc, x.timestamp)
    writePersistedScheduleEventKind(bc, x.kind)
}

export type PersistedHibernatableWebSocket = {
    readonly requestId: ArrayBuffer,
    readonly lastSeenTimestamp: i64,
    readonly msgIndex: i64,
}

export function readPersistedHibernatableWebSocket(bc: bare.ByteCursor): PersistedHibernatableWebSocket {
    return {
        requestId: bare.readData(bc),
        lastSeenTimestamp: bare.readI64(bc),
        msgIndex: bare.readI64(bc),
    }
}

export function writePersistedHibernatableWebSocket(bc: bare.ByteCursor, x: PersistedHibernatableWebSocket): void {
    bare.writeData(bc, x.requestId)
    bare.writeI64(bc, x.lastSeenTimestamp)
    bare.writeI64(bc, x.msgIndex)
}

function read2(bc: bare.ByteCursor): readonly PersistedConnection[] {
    const len = bare.readUintSafe(bc)
    if (len === 0) { return [] }
    const result = [readPersistedConnection(bc)]
    for (let i = 1; i < len; i++) {
        result[i] = readPersistedConnection(bc)
    }
    return result
}

function write2(bc: bare.ByteCursor, x: readonly PersistedConnection[]): void {
    bare.writeUintSafe(bc, x.length)
    for (let i = 0; i < x.length; i++) {
        writePersistedConnection(bc, x[i])
    }
}

function read3(bc: bare.ByteCursor): readonly PersistedScheduleEvent[] {
    const len = bare.readUintSafe(bc)
    if (len === 0) { return [] }
    const result = [readPersistedScheduleEvent(bc)]
    for (let i = 1; i < len; i++) {
        result[i] = readPersistedScheduleEvent(bc)
    }
    return result
}

function write3(bc: bare.ByteCursor, x: readonly PersistedScheduleEvent[]): void {
    bare.writeUintSafe(bc, x.length)
    for (let i = 0; i < x.length; i++) {
        writePersistedScheduleEvent(bc, x[i])
    }
}

function read4(bc: bare.ByteCursor): readonly PersistedHibernatableWebSocket[] {
    const len = bare.readUintSafe(bc)
    if (len === 0) { return [] }
    const result = [readPersistedHibernatableWebSocket(bc)]
    for (let i = 1; i < len; i++) {
        result[i] = readPersistedHibernatableWebSocket(bc)
    }
    return result
}

function write4(bc: bare.ByteCursor, x: readonly PersistedHibernatableWebSocket[]): void {
    bare.writeUintSafe(bc, x.length)
    for (let i = 0; i < x.length; i++) {
        writePersistedHibernatableWebSocket(bc, x[i])
    }
}

export type PersistedActor = {
    readonly input: ArrayBuffer | null,
    readonly hasInitialized: boolean,
    readonly state: ArrayBuffer,
    readonly connections: readonly PersistedConnection[],
    readonly scheduledEvents: readonly PersistedScheduleEvent[],
    readonly hibernatableWebSockets: readonly PersistedHibernatableWebSocket[],
}

export function readPersistedActor(bc: bare.ByteCursor): PersistedActor {
    return {
        input: read1(bc),
        hasInitialized: bare.readBool(bc),
        state: bare.readData(bc),
        connections: read2(bc),
        scheduledEvents: read3(bc),
        hibernatableWebSockets: read4(bc),
    }
}

export function writePersistedActor(bc: bare.ByteCursor, x: PersistedActor): void {
    write1(bc, x.input)
    bare.writeBool(bc, x.hasInitialized)
    bare.writeData(bc, x.state)
    write2(bc, x.connections)
    write3(bc, x.scheduledEvents)
    write4(bc, x.hibernatableWebSockets)
}

export function encodePersistedActor(x: PersistedActor): Uint8Array {
    const bc = new bare.ByteCursor(
        new Uint8Array(config.initialBufferLength),
        config
    )
    writePersistedActor(bc, x)
    return new Uint8Array(bc.view.buffer, bc.view.byteOffset, bc.offset)
}

export function decodePersistedActor(bytes: Uint8Array): PersistedActor {
    const bc = new bare.ByteCursor(bytes, config)
    const result = readPersistedActor(bc)
    if (bc.offset < bc.view.byteLength) {
        throw new bare.BareError(bc.offset, "remaining bytes")
    }
    return result
}


function assert(condition: boolean, message?: string): asserts condition {
    if (!condition) throw new Error(message ?? "Assertion failed")
}
