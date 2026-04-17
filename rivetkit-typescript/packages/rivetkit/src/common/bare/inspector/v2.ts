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

export type TraceQueryRequest = {
    readonly id: uint,
    readonly startMs: uint,
    readonly endMs: uint,
    readonly limit: uint,
}

export function readTraceQueryRequest(bc: bare.ByteCursor): TraceQueryRequest {
    return {
        id: bare.readUint(bc),
        startMs: bare.readUint(bc),
        endMs: bare.readUint(bc),
        limit: bare.readUint(bc),
    }
}

export function writeTraceQueryRequest(bc: bare.ByteCursor, x: TraceQueryRequest): void {
    bare.writeUint(bc, x.id)
    bare.writeUint(bc, x.startMs)
    bare.writeUint(bc, x.endMs)
    bare.writeUint(bc, x.limit)
}

export type QueueRequest = {
    readonly id: uint,
    readonly limit: uint,
}

export function readQueueRequest(bc: bare.ByteCursor): QueueRequest {
    return {
        id: bare.readUint(bc),
        limit: bare.readUint(bc),
    }
}

export function writeQueueRequest(bc: bare.ByteCursor, x: QueueRequest): void {
    bare.writeUint(bc, x.id)
    bare.writeUint(bc, x.limit)
}

export type WorkflowHistoryRequest = {
    readonly id: uint,
}

export function readWorkflowHistoryRequest(bc: bare.ByteCursor): WorkflowHistoryRequest {
    return {
        id: bare.readUint(bc),
    }
}

export function writeWorkflowHistoryRequest(bc: bare.ByteCursor, x: WorkflowHistoryRequest): void {
    bare.writeUint(bc, x.id)
}

export type ToServerBody =
    | { readonly tag: "PatchStateRequest", readonly val: PatchStateRequest }
    | { readonly tag: "StateRequest", readonly val: StateRequest }
    | { readonly tag: "ConnectionsRequest", readonly val: ConnectionsRequest }
    | { readonly tag: "ActionRequest", readonly val: ActionRequest }
    | { readonly tag: "RpcsListRequest", readonly val: RpcsListRequest }
    | { readonly tag: "TraceQueryRequest", readonly val: TraceQueryRequest }
    | { readonly tag: "QueueRequest", readonly val: QueueRequest }
    | { readonly tag: "WorkflowHistoryRequest", readonly val: WorkflowHistoryRequest }

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
            return { tag: "RpcsListRequest", val: readRpcsListRequest(bc) }
        case 5:
            return { tag: "TraceQueryRequest", val: readTraceQueryRequest(bc) }
        case 6:
            return { tag: "QueueRequest", val: readQueueRequest(bc) }
        case 7:
            return { tag: "WorkflowHistoryRequest", val: readWorkflowHistoryRequest(bc) }
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
        case "RpcsListRequest": {
            bare.writeU8(bc, 4)
            writeRpcsListRequest(bc, x.val)
            break
        }
        case "TraceQueryRequest": {
            bare.writeU8(bc, 5)
            writeTraceQueryRequest(bc, x.val)
            break
        }
        case "QueueRequest": {
            bare.writeU8(bc, 6)
            writeQueueRequest(bc, x.val)
            break
        }
        case "WorkflowHistoryRequest": {
            bare.writeU8(bc, 7)
            writeWorkflowHistoryRequest(bc, x.val)
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

export type WorkflowHistory = ArrayBuffer

export function readWorkflowHistory(bc: bare.ByteCursor): WorkflowHistory {
    return bare.readData(bc)
}

export function writeWorkflowHistory(bc: bare.ByteCursor, x: WorkflowHistory): void {
    bare.writeData(bc, x)
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

function read1(bc: bare.ByteCursor): State | null {
    return bare.readBool(bc)
        ? readState(bc)
        : null
}

function write1(bc: bare.ByteCursor, x: State | null): void {
    bare.writeBool(bc, x !== null)
    if (x !== null) {
        writeState(bc, x)
    }
}

function read2(bc: bare.ByteCursor): readonly string[] {
    const len = bare.readUintSafe(bc)
    if (len === 0) { return [] }
    const result = [bare.readString(bc)]
    for (let i = 1; i < len; i++) {
        result[i] = bare.readString(bc)
    }
    return result
}

function write2(bc: bare.ByteCursor, x: readonly string[]): void {
    bare.writeUintSafe(bc, x.length)
    for (let i = 0; i < x.length; i++) {
        bare.writeString(bc, x[i])
    }
}

function read3(bc: bare.ByteCursor): WorkflowHistory | null {
    return bare.readBool(bc)
        ? readWorkflowHistory(bc)
        : null
}

function write3(bc: bare.ByteCursor, x: WorkflowHistory | null): void {
    bare.writeBool(bc, x !== null)
    if (x !== null) {
        writeWorkflowHistory(bc, x)
    }
}

export type Init = {
    readonly connections: readonly Connection[],
    readonly state: State | null,
    readonly isStateEnabled: boolean,
    readonly rpcs: readonly string[],
    readonly isDatabaseEnabled: boolean,
    readonly queueSize: uint,
    readonly workflowHistory: WorkflowHistory | null,
    readonly isWorkflowEnabled: boolean,
}

export function readInit(bc: bare.ByteCursor): Init {
    return {
        connections: read0(bc),
        state: read1(bc),
        isStateEnabled: bare.readBool(bc),
        rpcs: read2(bc),
        isDatabaseEnabled: bare.readBool(bc),
        queueSize: bare.readUint(bc),
        workflowHistory: read3(bc),
        isWorkflowEnabled: bare.readBool(bc),
    }
}

export function writeInit(bc: bare.ByteCursor, x: Init): void {
    write0(bc, x.connections)
    write1(bc, x.state)
    bare.writeBool(bc, x.isStateEnabled)
    write2(bc, x.rpcs)
    bare.writeBool(bc, x.isDatabaseEnabled)
    bare.writeUint(bc, x.queueSize)
    write3(bc, x.workflowHistory)
    bare.writeBool(bc, x.isWorkflowEnabled)
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
        state: read1(bc),
        isStateEnabled: bare.readBool(bc),
    }
}

export function writeStateResponse(bc: bare.ByteCursor, x: StateResponse): void {
    bare.writeUint(bc, x.rid)
    write1(bc, x.state)
    bare.writeBool(bc, x.isStateEnabled)
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

export type TraceQueryResponse = {
    readonly rid: uint,
    readonly payload: ArrayBuffer,
}

export function readTraceQueryResponse(bc: bare.ByteCursor): TraceQueryResponse {
    return {
        rid: bare.readUint(bc),
        payload: bare.readData(bc),
    }
}

export function writeTraceQueryResponse(bc: bare.ByteCursor, x: TraceQueryResponse): void {
    bare.writeUint(bc, x.rid)
    bare.writeData(bc, x.payload)
}

export type QueueMessageSummary = {
    readonly id: uint,
    readonly name: string,
    readonly createdAtMs: uint,
}

export function readQueueMessageSummary(bc: bare.ByteCursor): QueueMessageSummary {
    return {
        id: bare.readUint(bc),
        name: bare.readString(bc),
        createdAtMs: bare.readUint(bc),
    }
}

export function writeQueueMessageSummary(bc: bare.ByteCursor, x: QueueMessageSummary): void {
    bare.writeUint(bc, x.id)
    bare.writeString(bc, x.name)
    bare.writeUint(bc, x.createdAtMs)
}

function read4(bc: bare.ByteCursor): readonly QueueMessageSummary[] {
    const len = bare.readUintSafe(bc)
    if (len === 0) { return [] }
    const result = [readQueueMessageSummary(bc)]
    for (let i = 1; i < len; i++) {
        result[i] = readQueueMessageSummary(bc)
    }
    return result
}

function write4(bc: bare.ByteCursor, x: readonly QueueMessageSummary[]): void {
    bare.writeUintSafe(bc, x.length)
    for (let i = 0; i < x.length; i++) {
        writeQueueMessageSummary(bc, x[i])
    }
}

export type QueueStatus = {
    readonly size: uint,
    readonly maxSize: uint,
    readonly messages: readonly QueueMessageSummary[],
    readonly truncated: boolean,
}

export function readQueueStatus(bc: bare.ByteCursor): QueueStatus {
    return {
        size: bare.readUint(bc),
        maxSize: bare.readUint(bc),
        messages: read4(bc),
        truncated: bare.readBool(bc),
    }
}

export function writeQueueStatus(bc: bare.ByteCursor, x: QueueStatus): void {
    bare.writeUint(bc, x.size)
    bare.writeUint(bc, x.maxSize)
    write4(bc, x.messages)
    bare.writeBool(bc, x.truncated)
}

export type QueueResponse = {
    readonly rid: uint,
    readonly status: QueueStatus,
}

export function readQueueResponse(bc: bare.ByteCursor): QueueResponse {
    return {
        rid: bare.readUint(bc),
        status: readQueueStatus(bc),
    }
}

export function writeQueueResponse(bc: bare.ByteCursor, x: QueueResponse): void {
    bare.writeUint(bc, x.rid)
    writeQueueStatus(bc, x.status)
}

export type WorkflowHistoryResponse = {
    readonly rid: uint,
    readonly history: WorkflowHistory | null,
    readonly isWorkflowEnabled: boolean,
}

export function readWorkflowHistoryResponse(bc: bare.ByteCursor): WorkflowHistoryResponse {
    return {
        rid: bare.readUint(bc),
        history: read3(bc),
        isWorkflowEnabled: bare.readBool(bc),
    }
}

export function writeWorkflowHistoryResponse(bc: bare.ByteCursor, x: WorkflowHistoryResponse): void {
    bare.writeUint(bc, x.rid)
    write3(bc, x.history)
    bare.writeBool(bc, x.isWorkflowEnabled)
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

export type QueueUpdated = {
    readonly queueSize: uint,
}

export function readQueueUpdated(bc: bare.ByteCursor): QueueUpdated {
    return {
        queueSize: bare.readUint(bc),
    }
}

export function writeQueueUpdated(bc: bare.ByteCursor, x: QueueUpdated): void {
    bare.writeUint(bc, x.queueSize)
}

export type WorkflowHistoryUpdated = {
    readonly history: WorkflowHistory,
}

export function readWorkflowHistoryUpdated(bc: bare.ByteCursor): WorkflowHistoryUpdated {
    return {
        history: readWorkflowHistory(bc),
    }
}

export function writeWorkflowHistoryUpdated(bc: bare.ByteCursor, x: WorkflowHistoryUpdated): void {
    writeWorkflowHistory(bc, x.history)
}

export type RpcsListResponse = {
    readonly rid: uint,
    readonly rpcs: readonly string[],
}

export function readRpcsListResponse(bc: bare.ByteCursor): RpcsListResponse {
    return {
        rid: bare.readUint(bc),
        rpcs: read2(bc),
    }
}

export function writeRpcsListResponse(bc: bare.ByteCursor, x: RpcsListResponse): void {
    bare.writeUint(bc, x.rid)
    write2(bc, x.rpcs)
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
    | { readonly tag: "ActionResponse", readonly val: ActionResponse }
    | { readonly tag: "ConnectionsUpdated", readonly val: ConnectionsUpdated }
    | { readonly tag: "QueueUpdated", readonly val: QueueUpdated }
    | { readonly tag: "StateUpdated", readonly val: StateUpdated }
    | { readonly tag: "WorkflowHistoryUpdated", readonly val: WorkflowHistoryUpdated }
    | { readonly tag: "RpcsListResponse", readonly val: RpcsListResponse }
    | { readonly tag: "TraceQueryResponse", readonly val: TraceQueryResponse }
    | { readonly tag: "QueueResponse", readonly val: QueueResponse }
    | { readonly tag: "WorkflowHistoryResponse", readonly val: WorkflowHistoryResponse }
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
            return { tag: "ActionResponse", val: readActionResponse(bc) }
        case 3:
            return { tag: "ConnectionsUpdated", val: readConnectionsUpdated(bc) }
        case 4:
            return { tag: "QueueUpdated", val: readQueueUpdated(bc) }
        case 5:
            return { tag: "StateUpdated", val: readStateUpdated(bc) }
        case 6:
            return { tag: "WorkflowHistoryUpdated", val: readWorkflowHistoryUpdated(bc) }
        case 7:
            return { tag: "RpcsListResponse", val: readRpcsListResponse(bc) }
        case 8:
            return { tag: "TraceQueryResponse", val: readTraceQueryResponse(bc) }
        case 9:
            return { tag: "QueueResponse", val: readQueueResponse(bc) }
        case 10:
            return { tag: "WorkflowHistoryResponse", val: readWorkflowHistoryResponse(bc) }
        case 11:
            return { tag: "Error", val: readError(bc) }
        case 12:
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
        case "ActionResponse": {
            bare.writeU8(bc, 2)
            writeActionResponse(bc, x.val)
            break
        }
        case "ConnectionsUpdated": {
            bare.writeU8(bc, 3)
            writeConnectionsUpdated(bc, x.val)
            break
        }
        case "QueueUpdated": {
            bare.writeU8(bc, 4)
            writeQueueUpdated(bc, x.val)
            break
        }
        case "StateUpdated": {
            bare.writeU8(bc, 5)
            writeStateUpdated(bc, x.val)
            break
        }
        case "WorkflowHistoryUpdated": {
            bare.writeU8(bc, 6)
            writeWorkflowHistoryUpdated(bc, x.val)
            break
        }
        case "RpcsListResponse": {
            bare.writeU8(bc, 7)
            writeRpcsListResponse(bc, x.val)
            break
        }
        case "TraceQueryResponse": {
            bare.writeU8(bc, 8)
            writeTraceQueryResponse(bc, x.val)
            break
        }
        case "QueueResponse": {
            bare.writeU8(bc, 9)
            writeQueueResponse(bc, x.val)
            break
        }
        case "WorkflowHistoryResponse": {
            bare.writeU8(bc, 10)
            writeWorkflowHistoryResponse(bc, x.val)
            break
        }
        case "Error": {
            bare.writeU8(bc, 11)
            writeError(bc, x.val)
            break
        }
        case "Init": {
            bare.writeU8(bc, 12)
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
