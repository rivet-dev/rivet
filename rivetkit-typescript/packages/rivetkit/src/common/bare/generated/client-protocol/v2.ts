// @generated - post-processed by build.rs
import * as bare from "@rivetkit/bare-ts"

const DEFAULT_CONFIG = /* @__PURE__ */ bare.Config({})

export type uint = bigint

export type Cbor = ArrayBuffer

export function readCbor(bc: bare.ByteCursor): Cbor {
    return bare.readData(bc)
}

export function writeCbor(bc: bare.ByteCursor, x: Cbor): void {
    bare.writeData(bc, x)
}

export type Init = {
    readonly actorId: string
    readonly connectionId: string
}

export function readInit(bc: bare.ByteCursor): Init {
    return {
        actorId: bare.readString(bc),
        connectionId: bare.readString(bc),
    }
}

export function writeInit(bc: bare.ByteCursor, x: Init): void {
    bare.writeString(bc, x.actorId)
    bare.writeString(bc, x.connectionId)
}

function read0(bc: bare.ByteCursor): Cbor | null {
    return bare.readBool(bc) ? readCbor(bc) : null
}

function write0(bc: bare.ByteCursor, x: Cbor | null): void {
    bare.writeBool(bc, x != null)
    if (x != null) {
        writeCbor(bc, x)
    }
}

function read1(bc: bare.ByteCursor): uint | null {
    return bare.readBool(bc) ? bare.readUint(bc) : null
}

function write1(bc: bare.ByteCursor, x: uint | null): void {
    bare.writeBool(bc, x != null)
    if (x != null) {
        bare.writeUint(bc, x)
    }
}

export type Error = {
    readonly group: string
    readonly code: string
    readonly message: string
    readonly metadata: Cbor | null
    readonly actionId: uint | null
}

export function readError(bc: bare.ByteCursor): Error {
    return {
        group: bare.readString(bc),
        code: bare.readString(bc),
        message: bare.readString(bc),
        metadata: read0(bc),
        actionId: read1(bc),
    }
}

export function writeError(bc: bare.ByteCursor, x: Error): void {
    bare.writeString(bc, x.group)
    bare.writeString(bc, x.code)
    bare.writeString(bc, x.message)
    write0(bc, x.metadata)
    write1(bc, x.actionId)
}

export type ActionResponse = {
    readonly id: uint
    readonly output: Cbor
}

export function readActionResponse(bc: bare.ByteCursor): ActionResponse {
    return {
        id: bare.readUint(bc),
        output: readCbor(bc),
    }
}

export function writeActionResponse(bc: bare.ByteCursor, x: ActionResponse): void {
    bare.writeUint(bc, x.id)
    writeCbor(bc, x.output)
}

export type Event = {
    readonly name: string
    readonly args: Cbor
}

export function readEvent(bc: bare.ByteCursor): Event {
    return {
        name: bare.readString(bc),
        args: readCbor(bc),
    }
}

export function writeEvent(bc: bare.ByteCursor, x: Event): void {
    bare.writeString(bc, x.name)
    writeCbor(bc, x.args)
}

export type ToClientBody =
    | { readonly tag: "Init"; readonly val: Init }
    | { readonly tag: "Error"; readonly val: Error }
    | { readonly tag: "ActionResponse"; readonly val: ActionResponse }
    | { readonly tag: "Event"; readonly val: Event }

export function readToClientBody(bc: bare.ByteCursor): ToClientBody {
    const offset = bc.offset
    const tag = bare.readU8(bc)
    switch (tag) {
        case 0:
            return { tag: "Init", val: readInit(bc) }
        case 1:
            return { tag: "Error", val: readError(bc) }
        case 2:
            return { tag: "ActionResponse", val: readActionResponse(bc) }
        case 3:
            return { tag: "Event", val: readEvent(bc) }
        default: {
            bc.offset = offset
            throw new bare.BareError(offset, "invalid tag")
        }
    }
}

export function writeToClientBody(bc: bare.ByteCursor, x: ToClientBody): void {
    switch (x.tag) {
        case "Init": {
            bare.writeU8(bc, 0)
            writeInit(bc, x.val)
            break
        }
        case "Error": {
            bare.writeU8(bc, 1)
            writeError(bc, x.val)
            break
        }
        case "ActionResponse": {
            bare.writeU8(bc, 2)
            writeActionResponse(bc, x.val)
            break
        }
        case "Event": {
            bare.writeU8(bc, 3)
            writeEvent(bc, x.val)
            break
        }
    }
}

export type ToClient = {
    readonly body: ToClientBody
}

export function readToClient(bc: bare.ByteCursor): ToClient {
    return {
        body: readToClientBody(bc),
    }
}

export function writeToClient(bc: bare.ByteCursor, x: ToClient): void {
    writeToClientBody(bc, x.body)
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

export type ActionRequest = {
    readonly id: uint
    readonly name: string
    readonly args: Cbor
}

export function readActionRequest(bc: bare.ByteCursor): ActionRequest {
    return {
        id: bare.readUint(bc),
        name: bare.readString(bc),
        args: readCbor(bc),
    }
}

export function writeActionRequest(bc: bare.ByteCursor, x: ActionRequest): void {
    bare.writeUint(bc, x.id)
    bare.writeString(bc, x.name)
    writeCbor(bc, x.args)
}

export type SubscriptionRequest = {
    readonly eventName: string
    readonly subscribe: boolean
}

export function readSubscriptionRequest(bc: bare.ByteCursor): SubscriptionRequest {
    return {
        eventName: bare.readString(bc),
        subscribe: bare.readBool(bc),
    }
}

export function writeSubscriptionRequest(bc: bare.ByteCursor, x: SubscriptionRequest): void {
    bare.writeString(bc, x.eventName)
    bare.writeBool(bc, x.subscribe)
}

export type ToServerBody =
    | { readonly tag: "ActionRequest"; readonly val: ActionRequest }
    | { readonly tag: "SubscriptionRequest"; readonly val: SubscriptionRequest }

export function readToServerBody(bc: bare.ByteCursor): ToServerBody {
    const offset = bc.offset
    const tag = bare.readU8(bc)
    switch (tag) {
        case 0:
            return { tag: "ActionRequest", val: readActionRequest(bc) }
        case 1:
            return { tag: "SubscriptionRequest", val: readSubscriptionRequest(bc) }
        default: {
            bc.offset = offset
            throw new bare.BareError(offset, "invalid tag")
        }
    }
}

export function writeToServerBody(bc: bare.ByteCursor, x: ToServerBody): void {
    switch (x.tag) {
        case "ActionRequest": {
            bare.writeU8(bc, 0)
            writeActionRequest(bc, x.val)
            break
        }
        case "SubscriptionRequest": {
            bare.writeU8(bc, 1)
            writeSubscriptionRequest(bc, x.val)
            break
        }
    }
}

export type ToServer = {
    readonly body: ToServerBody
}

export function readToServer(bc: bare.ByteCursor): ToServer {
    return {
        body: readToServerBody(bc),
    }
}

export function writeToServer(bc: bare.ByteCursor, x: ToServer): void {
    writeToServerBody(bc, x.body)
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

export type HttpActionRequest = {
    readonly args: Cbor
}

export function readHttpActionRequest(bc: bare.ByteCursor): HttpActionRequest {
    return {
        args: readCbor(bc),
    }
}

export function writeHttpActionRequest(bc: bare.ByteCursor, x: HttpActionRequest): void {
    writeCbor(bc, x.args)
}

export function encodeHttpActionRequest(x: HttpActionRequest, config?: Partial<bare.Config>): Uint8Array {
    const fullConfig = config != null ? bare.Config(config) : DEFAULT_CONFIG
    const bc = new bare.ByteCursor(
        new Uint8Array(fullConfig.initialBufferLength),
        fullConfig,
    )
    writeHttpActionRequest(bc, x)
    return new Uint8Array(bc.view.buffer, bc.view.byteOffset, bc.offset)
}

export function decodeHttpActionRequest(bytes: Uint8Array): HttpActionRequest {
    const bc = new bare.ByteCursor(bytes, DEFAULT_CONFIG)
    const result = readHttpActionRequest(bc)
    if (bc.offset < bc.view.byteLength) {
        throw new bare.BareError(bc.offset, "remaining bytes")
    }
    return result
}

export type HttpActionResponse = {
    readonly output: Cbor
}

export function readHttpActionResponse(bc: bare.ByteCursor): HttpActionResponse {
    return {
        output: readCbor(bc),
    }
}

export function writeHttpActionResponse(bc: bare.ByteCursor, x: HttpActionResponse): void {
    writeCbor(bc, x.output)
}

export function encodeHttpActionResponse(x: HttpActionResponse, config?: Partial<bare.Config>): Uint8Array {
    const fullConfig = config != null ? bare.Config(config) : DEFAULT_CONFIG
    const bc = new bare.ByteCursor(
        new Uint8Array(fullConfig.initialBufferLength),
        fullConfig,
    )
    writeHttpActionResponse(bc, x)
    return new Uint8Array(bc.view.buffer, bc.view.byteOffset, bc.offset)
}

export function decodeHttpActionResponse(bytes: Uint8Array): HttpActionResponse {
    const bc = new bare.ByteCursor(bytes, DEFAULT_CONFIG)
    const result = readHttpActionResponse(bc)
    if (bc.offset < bc.view.byteLength) {
        throw new bare.BareError(bc.offset, "remaining bytes")
    }
    return result
}

export type HttpResponseError = {
    readonly group: string
    readonly code: string
    readonly message: string
    readonly metadata: Cbor | null
}

export function readHttpResponseError(bc: bare.ByteCursor): HttpResponseError {
    return {
        group: bare.readString(bc),
        code: bare.readString(bc),
        message: bare.readString(bc),
        metadata: read0(bc),
    }
}

export function writeHttpResponseError(bc: bare.ByteCursor, x: HttpResponseError): void {
    bare.writeString(bc, x.group)
    bare.writeString(bc, x.code)
    bare.writeString(bc, x.message)
    write0(bc, x.metadata)
}

export function encodeHttpResponseError(x: HttpResponseError, config?: Partial<bare.Config>): Uint8Array {
    const fullConfig = config != null ? bare.Config(config) : DEFAULT_CONFIG
    const bc = new bare.ByteCursor(
        new Uint8Array(fullConfig.initialBufferLength),
        fullConfig,
    )
    writeHttpResponseError(bc, x)
    return new Uint8Array(bc.view.buffer, bc.view.byteOffset, bc.offset)
}

export function decodeHttpResponseError(bytes: Uint8Array): HttpResponseError {
    const bc = new bare.ByteCursor(bytes, DEFAULT_CONFIG)
    const result = readHttpResponseError(bc)
    if (bc.offset < bc.view.byteLength) {
        throw new bare.BareError(bc.offset, "remaining bytes")
    }
    return result
}

export type HttpResolveRequest = null

export type HttpResolveResponse = {
    readonly actorId: string
}

export function readHttpResolveResponse(bc: bare.ByteCursor): HttpResolveResponse {
    return {
        actorId: bare.readString(bc),
    }
}

export function writeHttpResolveResponse(bc: bare.ByteCursor, x: HttpResolveResponse): void {
    bare.writeString(bc, x.actorId)
}

export function encodeHttpResolveResponse(x: HttpResolveResponse, config?: Partial<bare.Config>): Uint8Array {
    const fullConfig = config != null ? bare.Config(config) : DEFAULT_CONFIG
    const bc = new bare.ByteCursor(
        new Uint8Array(fullConfig.initialBufferLength),
        fullConfig,
    )
    writeHttpResolveResponse(bc, x)
    return new Uint8Array(bc.view.buffer, bc.view.byteOffset, bc.offset)
}

export function decodeHttpResolveResponse(bytes: Uint8Array): HttpResolveResponse {
    const bc = new bare.ByteCursor(bytes, DEFAULT_CONFIG)
    const result = readHttpResolveResponse(bc)
    if (bc.offset < bc.view.byteLength) {
        throw new bare.BareError(bc.offset, "remaining bytes")
    }
    return result
}


function assert(condition: boolean, message?: string): asserts condition {
    if (!condition) throw new Error(message ?? "Assertion failed")
}
