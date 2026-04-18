// @generated - post-processed by compile-bare.ts
import * as bare from "@rivetkit/bare-ts"

const config = /* @__PURE__ */ bare.Config({})

export type uint = bigint

export type PatchStateRequest = {
    readonly state: ArrayBuffer,
}

export function readPatchStateRequest(bc: bare.ByteCursor): PatchStateRequest {
    return {
        state: bare.readData(bc),
    }
}

export function writePatchStateRequest(bc: bare.ByteCursor, x: PatchStateRequest): void {
    bare.writeData(bc, x.state)
}

export type ActionRequest = {
    readonly id: uint,
    readonly name: string,
    readonly args: ArrayBuffer,
}

export function readActionRequest(bc: bare.ByteCursor): ActionRequest {
    return {
        id: bare.readUint(bc),
        name: bare.readString(bc),
        args: bare.readData(bc),
    }
}

export function writeActionRequest(bc: bare.ByteCursor, x: ActionRequest): void {
    bare.writeUint(bc, x.id)
    bare.writeString(bc, x.name)
    bare.writeData(bc, x.args)
}

export type StateRequest = {
    readonly id: uint,
}

export function readStateRequest(bc: bare.ByteCursor): StateRequest {
    return {
        id: bare.readUint(bc),
    }
}

export function writeStateRequest(bc: bare.ByteCursor, x: StateRequest): void {
    bare.writeUint(bc, x.id)
}

export type ConnectionsRequest = {
    readonly id: uint,
}

export function readConnectionsRequest(bc: bare.ByteCursor): ConnectionsRequest {
    return {
        id: bare.readUint(bc),
    }
}

export function writeConnectionsRequest(bc: bare.ByteCursor, x: ConnectionsRequest): void {
    bare.writeUint(bc, x.id)
}

export type EventsRequest = {
    readonly id: uint,
}

export function readEventsRequest(bc: bare.ByteCursor): EventsRequest {
    return {
        id: bare.readUint(bc),
    }
}

export function writeEventsRequest(bc: bare.ByteCursor, x: EventsRequest): void {
    bare.writeUint(bc, x.id)
}

export type ClearEventsRequest = {
    readonly id: uint,
}

export function readClearEventsRequest(bc: bare.ByteCursor): ClearEventsRequest {
    return {
        id: bare.readUint(bc),
    }
}

export function writeClearEventsRequest(bc: bare.ByteCursor, x: ClearEventsRequest): void {
    bare.writeUint(bc, x.id)
}

export type RpcsListRequest = {
    readonly id: uint,
}

export function readRpcsListRequest(bc: bare.ByteCursor): RpcsListRequest {
    return {
        id: bare.readUint(bc),
    }
}

export function writeRpcsListRequest(bc: bare.ByteCursor, x: RpcsListRequest): void {
    bare.writeUint(bc, x.id)
}

export type ToServerBody =
    | { readonly tag: "PatchStateRequest", readonly val: PatchStateRequest }
    | { readonly tag: "StateRequest", readonly val: StateRequest }
    | { readonly tag: "ConnectionsRequest", readonly val: ConnectionsRequest }
    | { readonly tag: "ActionRequest", readonly val: ActionRequest }
    | { readonly tag: "EventsRequest", readonly val: EventsRequest }
    | { readonly tag: "ClearEventsRequest", readonly val: ClearEventsRequest }
    | { readonly tag: "RpcsListRequest", readonly val: RpcsListRequest }

export function readToServerBody(bc: bare.ByteCursor): ToServerBody {
    const offset = bc.offset
    const tag = bare.readU8(bc)
    switch (tag) {
        case 0:
            return { tag: "PatchStateRequest", val: readPatchStateRequest(bc) }
        case 1:
            return { tag: "StateRequest", val: readStateRequest(bc) }
        case 2:
            return { tag: "ConnectionsRequest", val: readConnectionsRequest(bc) }
        case 3:
            return { tag: "ActionRequest", val: readActionRequest(bc) }
        case 4:
            return { tag: "EventsRequest", val: readEventsRequest(bc) }
        case 5:
            return { tag: "ClearEventsRequest", val: readClearEventsRequest(bc) }
        case 6:
            return { tag: "RpcsListRequest", val: readRpcsListRequest(bc) }
        default: {
            bc.offset = offset
            throw new bare.BareError(offset, "invalid tag")
        }
    }
}

export function writeToServerBody(bc: bare.ByteCursor, x: ToServerBody): void {
    switch (x.tag) {
        case "PatchStateRequest": {
            bare.writeU8(bc, 0)
            writePatchStateRequest(bc, x.val)
            break
        }
        case "StateRequest": {
            bare.writeU8(bc, 1)
            writeStateRequest(bc, x.val)
            break
        }
        case "ConnectionsRequest": {
            bare.writeU8(bc, 2)
            writeConnectionsRequest(bc, x.val)
            break
        }
        case "ActionRequest": {
            bare.writeU8(bc, 3)
            writeActionRequest(bc, x.val)
            break
        }
        case "EventsRequest": {
            bare.writeU8(bc, 4)
            writeEventsRequest(bc, x.val)
            break
        }
        case "ClearEventsRequest": {
            bare.writeU8(bc, 5)
            writeClearEventsRequest(bc, x.val)
            break
        }
        case "RpcsListRequest": {
            bare.writeU8(bc, 6)
            writeRpcsListRequest(bc, x.val)
            break
        }
    }
}

export type ToServer = {
    readonly body: ToServerBody,
}

export function readToServer(bc: bare.ByteCursor): ToServer {
    return {
        body: readToServerBody(bc),
    }
}

export function writeToServer(bc: bare.ByteCursor, x: ToServer): void {
    writeToServerBody(bc, x.body)
}

export function encodeToServer(x: ToServer): Uint8Array {
    const bc = new bare.ByteCursor(
        new Uint8Array(config.initialBufferLength),
        config
    )
    writeToServer(bc, x)
    return new Uint8Array(bc.view.buffer, bc.view.byteOffset, bc.offset)
}

export function decodeToServer(bytes: Uint8Array): ToServer {
    const bc = new bare.ByteCursor(bytes, config)
    const result = readToServer(bc)
    if (bc.offset < bc.view.byteLength) {
        throw new bare.BareError(bc.offset, "remaining bytes")
    }
    return result
}

export type State = ArrayBuffer

export function readState(bc: bare.ByteCursor): State {
    return bare.readData(bc)
}

export function writeState(bc: bare.ByteCursor, x: State): void {
    bare.writeData(bc, x)
}

export type Connection = {
    readonly id: string,
    readonly details: ArrayBuffer,
}

export function readConnection(bc: bare.ByteCursor): Connection {
    return {
        id: bare.readString(bc),
        details: bare.readData(bc),
    }
}

export function writeConnection(bc: bare.ByteCursor, x: Connection): void {
    bare.writeString(bc, x.id)
    bare.writeData(bc, x.details)
}

export type ActionEvent = {
    readonly name: string,
    readonly args: ArrayBuffer,
    readonly connId: string,
}

export function readActionEvent(bc: bare.ByteCursor): ActionEvent {
    return {
        name: bare.readString(bc),
        args: bare.readData(bc),
        connId: bare.readString(bc),
    }
}

export function writeActionEvent(bc: bare.ByteCursor, x: ActionEvent): void {
    bare.writeString(bc, x.name)
    bare.writeData(bc, x.args)
    bare.writeString(bc, x.connId)
}

export type BroadcastEvent = {
    readonly eventName: string,
    readonly args: ArrayBuffer,
}

export function readBroadcastEvent(bc: bare.ByteCursor): BroadcastEvent {
    return {
        eventName: bare.readString(bc),
        args: bare.readData(bc),
    }
}

export function writeBroadcastEvent(bc: bare.ByteCursor, x: BroadcastEvent): void {
    bare.writeString(bc, x.eventName)
    bare.writeData(bc, x.args)
}

export type SubscribeEvent = {
    readonly eventName: string,
    readonly connId: string,
}

export function readSubscribeEvent(bc: bare.ByteCursor): SubscribeEvent {
    return {
        eventName: bare.readString(bc),
        connId: bare.readString(bc),
    }
}

export function writeSubscribeEvent(bc: bare.ByteCursor, x: SubscribeEvent): void {
    bare.writeString(bc, x.eventName)
    bare.writeString(bc, x.connId)
}

export type UnSubscribeEvent = {
    readonly eventName: string,
    readonly connId: string,
}

export function readUnSubscribeEvent(bc: bare.ByteCursor): UnSubscribeEvent {
    return {
        eventName: bare.readString(bc),
        connId: bare.readString(bc),
    }
}

export function writeUnSubscribeEvent(bc: bare.ByteCursor, x: UnSubscribeEvent): void {
    bare.writeString(bc, x.eventName)
    bare.writeString(bc, x.connId)
}

export type FiredEvent = {
    readonly eventName: string,
    readonly args: ArrayBuffer,
    readonly connId: string,
}

export function readFiredEvent(bc: bare.ByteCursor): FiredEvent {
    return {
        eventName: bare.readString(bc),
        args: bare.readData(bc),
        connId: bare.readString(bc),
    }
}

export function writeFiredEvent(bc: bare.ByteCursor, x: FiredEvent): void {
    bare.writeString(bc, x.eventName)
    bare.writeData(bc, x.args)
    bare.writeString(bc, x.connId)
}

export type EventBody =
    | { readonly tag: "ActionEvent", readonly val: ActionEvent }
    | { readonly tag: "BroadcastEvent", readonly val: BroadcastEvent }
    | { readonly tag: "SubscribeEvent", readonly val: SubscribeEvent }
    | { readonly tag: "UnSubscribeEvent", readonly val: UnSubscribeEvent }
    | { readonly tag: "FiredEvent", readonly val: FiredEvent }

export function readEventBody(bc: bare.ByteCursor): EventBody {
    const offset = bc.offset
    const tag = bare.readU8(bc)
    switch (tag) {
        case 0:
            return { tag: "ActionEvent", val: readActionEvent(bc) }
        case 1:
            return { tag: "BroadcastEvent", val: readBroadcastEvent(bc) }
        case 2:
            return { tag: "SubscribeEvent", val: readSubscribeEvent(bc) }
        case 3:
            return { tag: "UnSubscribeEvent", val: readUnSubscribeEvent(bc) }
        case 4:
            return { tag: "FiredEvent", val: readFiredEvent(bc) }
        default: {
            bc.offset = offset
            throw new bare.BareError(offset, "invalid tag")
        }
    }
}

export function writeEventBody(bc: bare.ByteCursor, x: EventBody): void {
    switch (x.tag) {
        case "ActionEvent": {
            bare.writeU8(bc, 0)
            writeActionEvent(bc, x.val)
            break
        }
        case "BroadcastEvent": {
            bare.writeU8(bc, 1)
            writeBroadcastEvent(bc, x.val)
            break
        }
        case "SubscribeEvent": {
            bare.writeU8(bc, 2)
            writeSubscribeEvent(bc, x.val)
            break
        }
        case "UnSubscribeEvent": {
            bare.writeU8(bc, 3)
            writeUnSubscribeEvent(bc, x.val)
            break
        }
        case "FiredEvent": {
            bare.writeU8(bc, 4)
            writeFiredEvent(bc, x.val)
            break
        }
    }
}

export type Event = {
    readonly id: string,
    readonly timestamp: uint,
    readonly body: EventBody,
}

export function readEvent(bc: bare.ByteCursor): Event {
    return {
        id: bare.readString(bc),
        timestamp: bare.readUint(bc),
        body: readEventBody(bc),
    }
}

export function writeEvent(bc: bare.ByteCursor, x: Event): void {
    bare.writeString(bc, x.id)
    bare.writeUint(bc, x.timestamp)
    writeEventBody(bc, x.body)
}

function read0(bc: bare.ByteCursor): readonly Connection[] {
    const len = bare.readUintSafe(bc)
    if (len === 0) { return [] }
    const result = [readConnection(bc)]
    for (let i = 1; i < len; i++) {
        result[i] = readConnection(bc)
    }
    return result
}

function write0(bc: bare.ByteCursor, x: readonly Connection[]): void {
    bare.writeUintSafe(bc, x.length)
    for (let i = 0; i < x.length; i++) {
        writeConnection(bc, x[i])
    }
}

function read1(bc: bare.ByteCursor): readonly Event[] {
    const len = bare.readUintSafe(bc)
    if (len === 0) { return [] }
    const result = [readEvent(bc)]
    for (let i = 1; i < len; i++) {
        result[i] = readEvent(bc)
    }
    return result
}

function write1(bc: bare.ByteCursor, x: readonly Event[]): void {
    bare.writeUintSafe(bc, x.length)
    for (let i = 0; i < x.length; i++) {
        writeEvent(bc, x[i])
    }
}

function read2(bc: bare.ByteCursor): State | null {
    return bare.readBool(bc)
        ? readState(bc)
        : null
}

function write2(bc: bare.ByteCursor, x: State | null): void {
    bare.writeBool(bc, x !== null)
    if (x !== null) {
        writeState(bc, x)
    }
}

function read3(bc: bare.ByteCursor): readonly string[] {
    const len = bare.readUintSafe(bc)
    if (len === 0) { return [] }
    const result = [bare.readString(bc)]
    for (let i = 1; i < len; i++) {
        result[i] = bare.readString(bc)
    }
    return result
}

function write3(bc: bare.ByteCursor, x: readonly string[]): void {
    bare.writeUintSafe(bc, x.length)
    for (let i = 0; i < x.length; i++) {
        bare.writeString(bc, x[i])
    }
}

export type Init = {
    readonly connections: readonly Connection[],
    readonly events: readonly Event[],
    readonly state: State | null,
    readonly isStateEnabled: boolean,
    readonly rpcs: readonly string[],
    readonly isDatabaseEnabled: boolean,
}

export function readInit(bc: bare.ByteCursor): Init {
    return {
        connections: read0(bc),
        events: read1(bc),
        state: read2(bc),
        isStateEnabled: bare.readBool(bc),
        rpcs: read3(bc),
        isDatabaseEnabled: bare.readBool(bc),
    }
}

export function writeInit(bc: bare.ByteCursor, x: Init): void {
    write0(bc, x.connections)
    write1(bc, x.events)
    write2(bc, x.state)
    bare.writeBool(bc, x.isStateEnabled)
    write3(bc, x.rpcs)
    bare.writeBool(bc, x.isDatabaseEnabled)
}

export type ConnectionsResponse = {
    readonly rid: uint,
    readonly connections: readonly Connection[],
}

export function readConnectionsResponse(bc: bare.ByteCursor): ConnectionsResponse {
    return {
        rid: bare.readUint(bc),
        connections: read0(bc),
    }
}

export function writeConnectionsResponse(bc: bare.ByteCursor, x: ConnectionsResponse): void {
    bare.writeUint(bc, x.rid)
    write0(bc, x.connections)
}

export type StateResponse = {
    readonly rid: uint,
    readonly state: State | null,
    readonly isStateEnabled: boolean,
}

export function readStateResponse(bc: bare.ByteCursor): StateResponse {
    return {
        rid: bare.readUint(bc),
        state: read2(bc),
        isStateEnabled: bare.readBool(bc),
    }
}

export function writeStateResponse(bc: bare.ByteCursor, x: StateResponse): void {
    bare.writeUint(bc, x.rid)
    write2(bc, x.state)
    bare.writeBool(bc, x.isStateEnabled)
}

export type EventsResponse = {
    readonly rid: uint,
    readonly events: readonly Event[],
}

export function readEventsResponse(bc: bare.ByteCursor): EventsResponse {
    return {
        rid: bare.readUint(bc),
        events: read1(bc),
    }
}

export function writeEventsResponse(bc: bare.ByteCursor, x: EventsResponse): void {
    bare.writeUint(bc, x.rid)
    write1(bc, x.events)
}

export type ActionResponse = {
    readonly rid: uint,
    readonly output: ArrayBuffer,
}

export function readActionResponse(bc: bare.ByteCursor): ActionResponse {
    return {
        rid: bare.readUint(bc),
        output: bare.readData(bc),
    }
}

export function writeActionResponse(bc: bare.ByteCursor, x: ActionResponse): void {
    bare.writeUint(bc, x.rid)
    bare.writeData(bc, x.output)
}

export type StateUpdated = {
    readonly state: State,
}

export function readStateUpdated(bc: bare.ByteCursor): StateUpdated {
    return {
        state: readState(bc),
    }
}

export function writeStateUpdated(bc: bare.ByteCursor, x: StateUpdated): void {
    writeState(bc, x.state)
}

export type EventsUpdated = {
    readonly events: readonly Event[],
}

export function readEventsUpdated(bc: bare.ByteCursor): EventsUpdated {
    return {
        events: read1(bc),
    }
}

export function writeEventsUpdated(bc: bare.ByteCursor, x: EventsUpdated): void {
    write1(bc, x.events)
}

export type RpcsListResponse = {
    readonly rid: uint,
    readonly rpcs: readonly string[],
}

export function readRpcsListResponse(bc: bare.ByteCursor): RpcsListResponse {
    return {
        rid: bare.readUint(bc),
        rpcs: read3(bc),
    }
}

export function writeRpcsListResponse(bc: bare.ByteCursor, x: RpcsListResponse): void {
    bare.writeUint(bc, x.rid)
    write3(bc, x.rpcs)
}

export type ConnectionsUpdated = {
    readonly connections: readonly Connection[],
}

export function readConnectionsUpdated(bc: bare.ByteCursor): ConnectionsUpdated {
    return {
        connections: read0(bc),
    }
}

export function writeConnectionsUpdated(bc: bare.ByteCursor, x: ConnectionsUpdated): void {
    write0(bc, x.connections)
}

export type Error = {
    readonly message: string,
}

export function readError(bc: bare.ByteCursor): Error {
    return {
        message: bare.readString(bc),
    }
}

export function writeError(bc: bare.ByteCursor, x: Error): void {
    bare.writeString(bc, x.message)
}

export type ToClientBody =
    | { readonly tag: "StateResponse", readonly val: StateResponse }
    | { readonly tag: "ConnectionsResponse", readonly val: ConnectionsResponse }
    | { readonly tag: "EventsResponse", readonly val: EventsResponse }
    | { readonly tag: "ActionResponse", readonly val: ActionResponse }
    | { readonly tag: "ConnectionsUpdated", readonly val: ConnectionsUpdated }
    | { readonly tag: "EventsUpdated", readonly val: EventsUpdated }
    | { readonly tag: "StateUpdated", readonly val: StateUpdated }
    | { readonly tag: "RpcsListResponse", readonly val: RpcsListResponse }
    | { readonly tag: "Error", readonly val: Error }
    | { readonly tag: "Init", readonly val: Init }

export function readToClientBody(bc: bare.ByteCursor): ToClientBody {
    const offset = bc.offset
    const tag = bare.readU8(bc)
    switch (tag) {
        case 0:
            return { tag: "StateResponse", val: readStateResponse(bc) }
        case 1:
            return { tag: "ConnectionsResponse", val: readConnectionsResponse(bc) }
        case 2:
            return { tag: "EventsResponse", val: readEventsResponse(bc) }
        case 3:
            return { tag: "ActionResponse", val: readActionResponse(bc) }
        case 4:
            return { tag: "ConnectionsUpdated", val: readConnectionsUpdated(bc) }
        case 5:
            return { tag: "EventsUpdated", val: readEventsUpdated(bc) }
        case 6:
            return { tag: "StateUpdated", val: readStateUpdated(bc) }
        case 7:
            return { tag: "RpcsListResponse", val: readRpcsListResponse(bc) }
        case 8:
            return { tag: "Error", val: readError(bc) }
        case 9:
            return { tag: "Init", val: readInit(bc) }
        default: {
            bc.offset = offset
            throw new bare.BareError(offset, "invalid tag")
        }
    }
}

export function writeToClientBody(bc: bare.ByteCursor, x: ToClientBody): void {
    switch (x.tag) {
        case "StateResponse": {
            bare.writeU8(bc, 0)
            writeStateResponse(bc, x.val)
            break
        }
        case "ConnectionsResponse": {
            bare.writeU8(bc, 1)
            writeConnectionsResponse(bc, x.val)
            break
        }
        case "EventsResponse": {
            bare.writeU8(bc, 2)
            writeEventsResponse(bc, x.val)
            break
        }
        case "ActionResponse": {
            bare.writeU8(bc, 3)
            writeActionResponse(bc, x.val)
            break
        }
        case "ConnectionsUpdated": {
            bare.writeU8(bc, 4)
            writeConnectionsUpdated(bc, x.val)
            break
        }
        case "EventsUpdated": {
            bare.writeU8(bc, 5)
            writeEventsUpdated(bc, x.val)
            break
        }
        case "StateUpdated": {
            bare.writeU8(bc, 6)
            writeStateUpdated(bc, x.val)
            break
        }
        case "RpcsListResponse": {
            bare.writeU8(bc, 7)
            writeRpcsListResponse(bc, x.val)
            break
        }
        case "Error": {
            bare.writeU8(bc, 8)
            writeError(bc, x.val)
            break
        }
        case "Init": {
            bare.writeU8(bc, 9)
            writeInit(bc, x.val)
            break
        }
    }
}

export type ToClient = {
    readonly body: ToClientBody,
}

export function readToClient(bc: bare.ByteCursor): ToClient {
    return {
        body: readToClientBody(bc),
    }
}

export function writeToClient(bc: bare.ByteCursor, x: ToClient): void {
    writeToClientBody(bc, x.body)
}

export function encodeToClient(x: ToClient): Uint8Array {
    const bc = new bare.ByteCursor(
        new Uint8Array(config.initialBufferLength),
        config
    )
    writeToClient(bc, x)
    return new Uint8Array(bc.view.buffer, bc.view.byteOffset, bc.offset)
}

export function decodeToClient(bytes: Uint8Array): ToClient {
    const bc = new bare.ByteCursor(bytes, config)
    const result = readToClient(bc)
    if (bc.offset < bc.view.byteLength) {
        throw new bare.BareError(bc.offset, "remaining bytes")
    }
    return result
}


function assert(condition: boolean, message?: string): asserts condition {
    if (!condition) throw new Error(message ?? "Assertion failed")
}
