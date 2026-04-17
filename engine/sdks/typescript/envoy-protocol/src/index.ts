// @generated - post-processed by build.rs

import * as bare from "@rivetkit/bare-ts"

const DEFAULT_CONFIG = /* @__PURE__ */ bare.Config({})

export type i64 = bigint
export type u16 = number
export type u32 = number
export type u64 = bigint

export type Id = string

export function readId(bc: bare.ByteCursor): Id {
    return bare.readString(bc)
}

export function writeId(bc: bare.ByteCursor, x: Id): void {
    bare.writeString(bc, x)
}

export type Json = string

export function readJson(bc: bare.ByteCursor): Json {
    return bare.readString(bc)
}

export function writeJson(bc: bare.ByteCursor, x: Json): void {
    bare.writeString(bc, x)
}

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

/**
 * Basic types
 */
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

export type KvMetadata = {
    readonly version: ArrayBuffer
    readonly updateTs: i64
}

export function readKvMetadata(bc: bare.ByteCursor): KvMetadata {
    return {
        version: bare.readData(bc),
        updateTs: bare.readI64(bc),
    }
}

export function writeKvMetadata(bc: bare.ByteCursor, x: KvMetadata): void {
    bare.writeData(bc, x.version)
    bare.writeI64(bc, x.updateTs)
}

/**
 * Query types
 */
export type KvListAllQuery = null

export type KvListRangeQuery = {
    readonly start: KvKey
    readonly end: KvKey
    readonly exclusive: boolean
}

export function readKvListRangeQuery(bc: bare.ByteCursor): KvListRangeQuery {
    return {
        start: readKvKey(bc),
        end: readKvKey(bc),
        exclusive: bare.readBool(bc),
    }
}

export function writeKvListRangeQuery(bc: bare.ByteCursor, x: KvListRangeQuery): void {
    writeKvKey(bc, x.start)
    writeKvKey(bc, x.end)
    bare.writeBool(bc, x.exclusive)
}

export type KvListPrefixQuery = {
    readonly key: KvKey
}

export function readKvListPrefixQuery(bc: bare.ByteCursor): KvListPrefixQuery {
    return {
        key: readKvKey(bc),
    }
}

export function writeKvListPrefixQuery(bc: bare.ByteCursor, x: KvListPrefixQuery): void {
    writeKvKey(bc, x.key)
}

export type KvListQuery =
    | { readonly tag: "KvListAllQuery"; readonly val: KvListAllQuery }
    | { readonly tag: "KvListRangeQuery"; readonly val: KvListRangeQuery }
    | { readonly tag: "KvListPrefixQuery"; readonly val: KvListPrefixQuery }

export function readKvListQuery(bc: bare.ByteCursor): KvListQuery {
    const offset = bc.offset
    const tag = bare.readU8(bc)
    switch (tag) {
        case 0:
            return { tag: "KvListAllQuery", val: null }
        case 1:
            return { tag: "KvListRangeQuery", val: readKvListRangeQuery(bc) }
        case 2:
            return { tag: "KvListPrefixQuery", val: readKvListPrefixQuery(bc) }
        default: {
            bc.offset = offset
            throw new bare.BareError(offset, "invalid tag")
        }
    }
}

export function writeKvListQuery(bc: bare.ByteCursor, x: KvListQuery): void {
    switch (x.tag) {
        case "KvListAllQuery": {
            bare.writeU8(bc, 0)
            break
        }
        case "KvListRangeQuery": {
            bare.writeU8(bc, 1)
            writeKvListRangeQuery(bc, x.val)
            break
        }
        case "KvListPrefixQuery": {
            bare.writeU8(bc, 2)
            writeKvListPrefixQuery(bc, x.val)
            break
        }
    }
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

/**
 * Request types
 */
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

function read1(bc: bare.ByteCursor): boolean | null {
    return bare.readBool(bc) ? bare.readBool(bc) : null
}

function write1(bc: bare.ByteCursor, x: boolean | null): void {
    bare.writeBool(bc, x != null)
    if (x != null) {
        bare.writeBool(bc, x)
    }
}

function read2(bc: bare.ByteCursor): u64 | null {
    return bare.readBool(bc) ? bare.readU64(bc) : null
}

function write2(bc: bare.ByteCursor, x: u64 | null): void {
    bare.writeBool(bc, x != null)
    if (x != null) {
        bare.writeU64(bc, x)
    }
}

export type KvListRequest = {
    readonly query: KvListQuery
    readonly reverse: boolean | null
    readonly limit: u64 | null
}

export function readKvListRequest(bc: bare.ByteCursor): KvListRequest {
    return {
        query: readKvListQuery(bc),
        reverse: read1(bc),
        limit: read2(bc),
    }
}

export function writeKvListRequest(bc: bare.ByteCursor, x: KvListRequest): void {
    writeKvListQuery(bc, x.query)
    write1(bc, x.reverse)
    write2(bc, x.limit)
}

function read3(bc: bare.ByteCursor): readonly KvValue[] {
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

function write3(bc: bare.ByteCursor, x: readonly KvValue[]): void {
    bare.writeUintSafe(bc, x.length)
    for (let i = 0; i < x.length; i++) {
        writeKvValue(bc, x[i])
    }
}

export type KvPutRequest = {
    readonly keys: readonly KvKey[]
    readonly values: readonly KvValue[]
}

export function readKvPutRequest(bc: bare.ByteCursor): KvPutRequest {
    return {
        keys: read0(bc),
        values: read3(bc),
    }
}

export function writeKvPutRequest(bc: bare.ByteCursor, x: KvPutRequest): void {
    write0(bc, x.keys)
    write3(bc, x.values)
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

export type KvDropRequest = null

/**
 * Response types
 */
export type KvErrorResponse = {
    readonly message: string
}

export function readKvErrorResponse(bc: bare.ByteCursor): KvErrorResponse {
    return {
        message: bare.readString(bc),
    }
}

export function writeKvErrorResponse(bc: bare.ByteCursor, x: KvErrorResponse): void {
    bare.writeString(bc, x.message)
}

function read4(bc: bare.ByteCursor): readonly KvMetadata[] {
    const len = bare.readUintSafe(bc)
    if (len === 0) {
        return []
    }
    const result = [readKvMetadata(bc)]
    for (let i = 1; i < len; i++) {
        result[i] = readKvMetadata(bc)
    }
    return result
}

function write4(bc: bare.ByteCursor, x: readonly KvMetadata[]): void {
    bare.writeUintSafe(bc, x.length)
    for (let i = 0; i < x.length; i++) {
        writeKvMetadata(bc, x[i])
    }
}

export type KvGetResponse = {
    readonly keys: readonly KvKey[]
    readonly values: readonly KvValue[]
    readonly metadata: readonly KvMetadata[]
}

export function readKvGetResponse(bc: bare.ByteCursor): KvGetResponse {
    return {
        keys: read0(bc),
        values: read3(bc),
        metadata: read4(bc),
    }
}

export function writeKvGetResponse(bc: bare.ByteCursor, x: KvGetResponse): void {
    write0(bc, x.keys)
    write3(bc, x.values)
    write4(bc, x.metadata)
}

export type KvListResponse = {
    readonly keys: readonly KvKey[]
    readonly values: readonly KvValue[]
    readonly metadata: readonly KvMetadata[]
}

export function readKvListResponse(bc: bare.ByteCursor): KvListResponse {
    return {
        keys: read0(bc),
        values: read3(bc),
        metadata: read4(bc),
    }
}

export function writeKvListResponse(bc: bare.ByteCursor, x: KvListResponse): void {
    write0(bc, x.keys)
    write3(bc, x.values)
    write4(bc, x.metadata)
}

export type KvPutResponse = null

export type KvDeleteResponse = null

export type KvDropResponse = null

/**
 * Request/Response unions
 */
export type KvRequestData =
    | { readonly tag: "KvGetRequest"; readonly val: KvGetRequest }
    | { readonly tag: "KvListRequest"; readonly val: KvListRequest }
    | { readonly tag: "KvPutRequest"; readonly val: KvPutRequest }
    | { readonly tag: "KvDeleteRequest"; readonly val: KvDeleteRequest }
    | { readonly tag: "KvDeleteRangeRequest"; readonly val: KvDeleteRangeRequest }
    | { readonly tag: "KvDropRequest"; readonly val: KvDropRequest }

export function readKvRequestData(bc: bare.ByteCursor): KvRequestData {
    const offset = bc.offset
    const tag = bare.readU8(bc)
    switch (tag) {
        case 0:
            return { tag: "KvGetRequest", val: readKvGetRequest(bc) }
        case 1:
            return { tag: "KvListRequest", val: readKvListRequest(bc) }
        case 2:
            return { tag: "KvPutRequest", val: readKvPutRequest(bc) }
        case 3:
            return { tag: "KvDeleteRequest", val: readKvDeleteRequest(bc) }
        case 4:
            return { tag: "KvDeleteRangeRequest", val: readKvDeleteRangeRequest(bc) }
        case 5:
            return { tag: "KvDropRequest", val: null }
        default: {
            bc.offset = offset
            throw new bare.BareError(offset, "invalid tag")
        }
    }
}

export function writeKvRequestData(bc: bare.ByteCursor, x: KvRequestData): void {
    switch (x.tag) {
        case "KvGetRequest": {
            bare.writeU8(bc, 0)
            writeKvGetRequest(bc, x.val)
            break
        }
        case "KvListRequest": {
            bare.writeU8(bc, 1)
            writeKvListRequest(bc, x.val)
            break
        }
        case "KvPutRequest": {
            bare.writeU8(bc, 2)
            writeKvPutRequest(bc, x.val)
            break
        }
        case "KvDeleteRequest": {
            bare.writeU8(bc, 3)
            writeKvDeleteRequest(bc, x.val)
            break
        }
        case "KvDeleteRangeRequest": {
            bare.writeU8(bc, 4)
            writeKvDeleteRangeRequest(bc, x.val)
            break
        }
        case "KvDropRequest": {
            bare.writeU8(bc, 5)
            break
        }
    }
}

export type KvResponseData =
    | { readonly tag: "KvErrorResponse"; readonly val: KvErrorResponse }
    | { readonly tag: "KvGetResponse"; readonly val: KvGetResponse }
    | { readonly tag: "KvListResponse"; readonly val: KvListResponse }
    | { readonly tag: "KvPutResponse"; readonly val: KvPutResponse }
    | { readonly tag: "KvDeleteResponse"; readonly val: KvDeleteResponse }
    | { readonly tag: "KvDropResponse"; readonly val: KvDropResponse }

export function readKvResponseData(bc: bare.ByteCursor): KvResponseData {
    const offset = bc.offset
    const tag = bare.readU8(bc)
    switch (tag) {
        case 0:
            return { tag: "KvErrorResponse", val: readKvErrorResponse(bc) }
        case 1:
            return { tag: "KvGetResponse", val: readKvGetResponse(bc) }
        case 2:
            return { tag: "KvListResponse", val: readKvListResponse(bc) }
        case 3:
            return { tag: "KvPutResponse", val: null }
        case 4:
            return { tag: "KvDeleteResponse", val: null }
        case 5:
            return { tag: "KvDropResponse", val: null }
        default: {
            bc.offset = offset
            throw new bare.BareError(offset, "invalid tag")
        }
    }
}

export function writeKvResponseData(bc: bare.ByteCursor, x: KvResponseData): void {
    switch (x.tag) {
        case "KvErrorResponse": {
            bare.writeU8(bc, 0)
            writeKvErrorResponse(bc, x.val)
            break
        }
        case "KvGetResponse": {
            bare.writeU8(bc, 1)
            writeKvGetResponse(bc, x.val)
            break
        }
        case "KvListResponse": {
            bare.writeU8(bc, 2)
            writeKvListResponse(bc, x.val)
            break
        }
        case "KvPutResponse": {
            bare.writeU8(bc, 3)
            break
        }
        case "KvDeleteResponse": {
            bare.writeU8(bc, 4)
            break
        }
        case "KvDropResponse": {
            bare.writeU8(bc, 5)
            break
        }
    }
}

export type SqliteGeneration = u64

export function readSqliteGeneration(bc: bare.ByteCursor): SqliteGeneration {
    return bare.readU64(bc)
}

export function writeSqliteGeneration(bc: bare.ByteCursor, x: SqliteGeneration): void {
    bare.writeU64(bc, x)
}

export type SqliteTxid = u64

export function readSqliteTxid(bc: bare.ByteCursor): SqliteTxid {
    return bare.readU64(bc)
}

export function writeSqliteTxid(bc: bare.ByteCursor, x: SqliteTxid): void {
    bare.writeU64(bc, x)
}

export type SqlitePgno = u32

export function readSqlitePgno(bc: bare.ByteCursor): SqlitePgno {
    return bare.readU32(bc)
}

export function writeSqlitePgno(bc: bare.ByteCursor, x: SqlitePgno): void {
    bare.writeU32(bc, x)
}

export type SqliteStageId = u64

export function readSqliteStageId(bc: bare.ByteCursor): SqliteStageId {
    return bare.readU64(bc)
}

export function writeSqliteStageId(bc: bare.ByteCursor, x: SqliteStageId): void {
    bare.writeU64(bc, x)
}

export type SqlitePageBytes = ArrayBuffer

export function readSqlitePageBytes(bc: bare.ByteCursor): SqlitePageBytes {
    return bare.readData(bc)
}

export function writeSqlitePageBytes(bc: bare.ByteCursor, x: SqlitePageBytes): void {
    bare.writeData(bc, x)
}

export type SqliteMeta = {
    readonly schemaVersion: u32
    readonly generation: SqliteGeneration
    readonly headTxid: SqliteTxid
    readonly materializedTxid: SqliteTxid
    readonly dbSizePages: u32
    readonly pageSize: u32
    readonly creationTsMs: i64
    readonly maxDeltaBytes: u64
}

export function readSqliteMeta(bc: bare.ByteCursor): SqliteMeta {
    return {
        schemaVersion: bare.readU32(bc),
        generation: readSqliteGeneration(bc),
        headTxid: readSqliteTxid(bc),
        materializedTxid: readSqliteTxid(bc),
        dbSizePages: bare.readU32(bc),
        pageSize: bare.readU32(bc),
        creationTsMs: bare.readI64(bc),
        maxDeltaBytes: bare.readU64(bc),
    }
}

export function writeSqliteMeta(bc: bare.ByteCursor, x: SqliteMeta): void {
    bare.writeU32(bc, x.schemaVersion)
    writeSqliteGeneration(bc, x.generation)
    writeSqliteTxid(bc, x.headTxid)
    writeSqliteTxid(bc, x.materializedTxid)
    bare.writeU32(bc, x.dbSizePages)
    bare.writeU32(bc, x.pageSize)
    bare.writeI64(bc, x.creationTsMs)
    bare.writeU64(bc, x.maxDeltaBytes)
}

export type SqliteFenceMismatch = {
    readonly actualMeta: SqliteMeta
    readonly reason: string
}

export function readSqliteFenceMismatch(bc: bare.ByteCursor): SqliteFenceMismatch {
    return {
        actualMeta: readSqliteMeta(bc),
        reason: bare.readString(bc),
    }
}

export function writeSqliteFenceMismatch(bc: bare.ByteCursor, x: SqliteFenceMismatch): void {
    writeSqliteMeta(bc, x.actualMeta)
    bare.writeString(bc, x.reason)
}

export type SqliteDirtyPage = {
    readonly pgno: SqlitePgno
    readonly bytes: SqlitePageBytes
}

export function readSqliteDirtyPage(bc: bare.ByteCursor): SqliteDirtyPage {
    return {
        pgno: readSqlitePgno(bc),
        bytes: readSqlitePageBytes(bc),
    }
}

export function writeSqliteDirtyPage(bc: bare.ByteCursor, x: SqliteDirtyPage): void {
    writeSqlitePgno(bc, x.pgno)
    writeSqlitePageBytes(bc, x.bytes)
}

function read5(bc: bare.ByteCursor): SqlitePageBytes | null {
    return bare.readBool(bc) ? readSqlitePageBytes(bc) : null
}

function write5(bc: bare.ByteCursor, x: SqlitePageBytes | null): void {
    bare.writeBool(bc, x != null)
    if (x != null) {
        writeSqlitePageBytes(bc, x)
    }
}

export type SqliteFetchedPage = {
    readonly pgno: SqlitePgno
    readonly bytes: SqlitePageBytes | null
}

export function readSqliteFetchedPage(bc: bare.ByteCursor): SqliteFetchedPage {
    return {
        pgno: readSqlitePgno(bc),
        bytes: read5(bc),
    }
}

export function writeSqliteFetchedPage(bc: bare.ByteCursor, x: SqliteFetchedPage): void {
    writeSqlitePgno(bc, x.pgno)
    write5(bc, x.bytes)
}

function read6(bc: bare.ByteCursor): readonly SqlitePgno[] {
    const len = bare.readUintSafe(bc)
    if (len === 0) {
        return []
    }
    const result = [readSqlitePgno(bc)]
    for (let i = 1; i < len; i++) {
        result[i] = readSqlitePgno(bc)
    }
    return result
}

function write6(bc: bare.ByteCursor, x: readonly SqlitePgno[]): void {
    bare.writeUintSafe(bc, x.length)
    for (let i = 0; i < x.length; i++) {
        writeSqlitePgno(bc, x[i])
    }
}

export type SqliteGetPagesRequest = {
    readonly actorId: Id
    readonly generation: SqliteGeneration
    readonly pgnos: readonly SqlitePgno[]
}

export function readSqliteGetPagesRequest(bc: bare.ByteCursor): SqliteGetPagesRequest {
    return {
        actorId: readId(bc),
        generation: readSqliteGeneration(bc),
        pgnos: read6(bc),
    }
}

export function writeSqliteGetPagesRequest(bc: bare.ByteCursor, x: SqliteGetPagesRequest): void {
    writeId(bc, x.actorId)
    writeSqliteGeneration(bc, x.generation)
    write6(bc, x.pgnos)
}

function read7(bc: bare.ByteCursor): readonly SqliteFetchedPage[] {
    const len = bare.readUintSafe(bc)
    if (len === 0) {
        return []
    }
    const result = [readSqliteFetchedPage(bc)]
    for (let i = 1; i < len; i++) {
        result[i] = readSqliteFetchedPage(bc)
    }
    return result
}

function write7(bc: bare.ByteCursor, x: readonly SqliteFetchedPage[]): void {
    bare.writeUintSafe(bc, x.length)
    for (let i = 0; i < x.length; i++) {
        writeSqliteFetchedPage(bc, x[i])
    }
}

export type SqliteGetPagesOk = {
    readonly pages: readonly SqliteFetchedPage[]
    readonly meta: SqliteMeta
}

export function readSqliteGetPagesOk(bc: bare.ByteCursor): SqliteGetPagesOk {
    return {
        pages: read7(bc),
        meta: readSqliteMeta(bc),
    }
}

export function writeSqliteGetPagesOk(bc: bare.ByteCursor, x: SqliteGetPagesOk): void {
    write7(bc, x.pages)
    writeSqliteMeta(bc, x.meta)
}

export type SqliteErrorResponse = {
    readonly message: string
}

export function readSqliteErrorResponse(bc: bare.ByteCursor): SqliteErrorResponse {
    return {
        message: bare.readString(bc),
    }
}

export function writeSqliteErrorResponse(bc: bare.ByteCursor, x: SqliteErrorResponse): void {
    bare.writeString(bc, x.message)
}

export type SqliteGetPagesResponse =
    | { readonly tag: "SqliteGetPagesOk"; readonly val: SqliteGetPagesOk }
    | { readonly tag: "SqliteFenceMismatch"; readonly val: SqliteFenceMismatch }
    | { readonly tag: "SqliteErrorResponse"; readonly val: SqliteErrorResponse }

export function readSqliteGetPagesResponse(bc: bare.ByteCursor): SqliteGetPagesResponse {
    const offset = bc.offset
    const tag = bare.readU8(bc)
    switch (tag) {
        case 0:
            return { tag: "SqliteGetPagesOk", val: readSqliteGetPagesOk(bc) }
        case 1:
            return { tag: "SqliteFenceMismatch", val: readSqliteFenceMismatch(bc) }
        case 2:
            return { tag: "SqliteErrorResponse", val: readSqliteErrorResponse(bc) }
        default: {
            bc.offset = offset
            throw new bare.BareError(offset, "invalid tag")
        }
    }
}

export function writeSqliteGetPagesResponse(bc: bare.ByteCursor, x: SqliteGetPagesResponse): void {
    switch (x.tag) {
        case "SqliteGetPagesOk": {
            bare.writeU8(bc, 0)
            writeSqliteGetPagesOk(bc, x.val)
            break
        }
        case "SqliteFenceMismatch": {
            bare.writeU8(bc, 1)
            writeSqliteFenceMismatch(bc, x.val)
            break
        }
        case "SqliteErrorResponse": {
            bare.writeU8(bc, 2)
            writeSqliteErrorResponse(bc, x.val)
            break
        }
    }
}

function read8(bc: bare.ByteCursor): readonly SqliteDirtyPage[] {
    const len = bare.readUintSafe(bc)
    if (len === 0) {
        return []
    }
    const result = [readSqliteDirtyPage(bc)]
    for (let i = 1; i < len; i++) {
        result[i] = readSqliteDirtyPage(bc)
    }
    return result
}

function write8(bc: bare.ByteCursor, x: readonly SqliteDirtyPage[]): void {
    bare.writeUintSafe(bc, x.length)
    for (let i = 0; i < x.length; i++) {
        writeSqliteDirtyPage(bc, x[i])
    }
}

export type SqliteCommitRequest = {
    readonly actorId: Id
    readonly generation: SqliteGeneration
    readonly expectedHeadTxid: SqliteTxid
    readonly dirtyPages: readonly SqliteDirtyPage[]
    readonly newDbSizePages: u32
}

export function readSqliteCommitRequest(bc: bare.ByteCursor): SqliteCommitRequest {
    return {
        actorId: readId(bc),
        generation: readSqliteGeneration(bc),
        expectedHeadTxid: readSqliteTxid(bc),
        dirtyPages: read8(bc),
        newDbSizePages: bare.readU32(bc),
    }
}

export function writeSqliteCommitRequest(bc: bare.ByteCursor, x: SqliteCommitRequest): void {
    writeId(bc, x.actorId)
    writeSqliteGeneration(bc, x.generation)
    writeSqliteTxid(bc, x.expectedHeadTxid)
    write8(bc, x.dirtyPages)
    bare.writeU32(bc, x.newDbSizePages)
}

export type SqliteCommitOk = {
    readonly newHeadTxid: SqliteTxid
    readonly meta: SqliteMeta
}

export function readSqliteCommitOk(bc: bare.ByteCursor): SqliteCommitOk {
    return {
        newHeadTxid: readSqliteTxid(bc),
        meta: readSqliteMeta(bc),
    }
}

export function writeSqliteCommitOk(bc: bare.ByteCursor, x: SqliteCommitOk): void {
    writeSqliteTxid(bc, x.newHeadTxid)
    writeSqliteMeta(bc, x.meta)
}

export type SqliteCommitTooLarge = {
    readonly actualSizeBytes: u64
    readonly maxSizeBytes: u64
}

export function readSqliteCommitTooLarge(bc: bare.ByteCursor): SqliteCommitTooLarge {
    return {
        actualSizeBytes: bare.readU64(bc),
        maxSizeBytes: bare.readU64(bc),
    }
}

export function writeSqliteCommitTooLarge(bc: bare.ByteCursor, x: SqliteCommitTooLarge): void {
    bare.writeU64(bc, x.actualSizeBytes)
    bare.writeU64(bc, x.maxSizeBytes)
}

export type SqliteCommitResponse =
    | { readonly tag: "SqliteCommitOk"; readonly val: SqliteCommitOk }
    | { readonly tag: "SqliteFenceMismatch"; readonly val: SqliteFenceMismatch }
    | { readonly tag: "SqliteCommitTooLarge"; readonly val: SqliteCommitTooLarge }
    | { readonly tag: "SqliteErrorResponse"; readonly val: SqliteErrorResponse }

export function readSqliteCommitResponse(bc: bare.ByteCursor): SqliteCommitResponse {
    const offset = bc.offset
    const tag = bare.readU8(bc)
    switch (tag) {
        case 0:
            return { tag: "SqliteCommitOk", val: readSqliteCommitOk(bc) }
        case 1:
            return { tag: "SqliteFenceMismatch", val: readSqliteFenceMismatch(bc) }
        case 2:
            return { tag: "SqliteCommitTooLarge", val: readSqliteCommitTooLarge(bc) }
        case 3:
            return { tag: "SqliteErrorResponse", val: readSqliteErrorResponse(bc) }
        default: {
            bc.offset = offset
            throw new bare.BareError(offset, "invalid tag")
        }
    }
}

export function writeSqliteCommitResponse(bc: bare.ByteCursor, x: SqliteCommitResponse): void {
    switch (x.tag) {
        case "SqliteCommitOk": {
            bare.writeU8(bc, 0)
            writeSqliteCommitOk(bc, x.val)
            break
        }
        case "SqliteFenceMismatch": {
            bare.writeU8(bc, 1)
            writeSqliteFenceMismatch(bc, x.val)
            break
        }
        case "SqliteCommitTooLarge": {
            bare.writeU8(bc, 2)
            writeSqliteCommitTooLarge(bc, x.val)
            break
        }
        case "SqliteErrorResponse": {
            bare.writeU8(bc, 3)
            writeSqliteErrorResponse(bc, x.val)
            break
        }
    }
}

export type SqliteCommitStageBeginRequest = {
    readonly actorId: Id
    readonly generation: SqliteGeneration
}

export function readSqliteCommitStageBeginRequest(bc: bare.ByteCursor): SqliteCommitStageBeginRequest {
    return {
        actorId: readId(bc),
        generation: readSqliteGeneration(bc),
    }
}

export function writeSqliteCommitStageBeginRequest(bc: bare.ByteCursor, x: SqliteCommitStageBeginRequest): void {
    writeId(bc, x.actorId)
    writeSqliteGeneration(bc, x.generation)
}

export type SqliteCommitStageBeginOk = {
    readonly txid: SqliteTxid
}

export function readSqliteCommitStageBeginOk(bc: bare.ByteCursor): SqliteCommitStageBeginOk {
    return {
        txid: readSqliteTxid(bc),
    }
}

export function writeSqliteCommitStageBeginOk(bc: bare.ByteCursor, x: SqliteCommitStageBeginOk): void {
    writeSqliteTxid(bc, x.txid)
}

export type SqliteCommitStageBeginResponse =
    | { readonly tag: "SqliteCommitStageBeginOk"; readonly val: SqliteCommitStageBeginOk }
    | { readonly tag: "SqliteFenceMismatch"; readonly val: SqliteFenceMismatch }
    | { readonly tag: "SqliteErrorResponse"; readonly val: SqliteErrorResponse }

export function readSqliteCommitStageBeginResponse(bc: bare.ByteCursor): SqliteCommitStageBeginResponse {
    const offset = bc.offset
    const tag = bare.readU8(bc)
    switch (tag) {
        case 0:
            return { tag: "SqliteCommitStageBeginOk", val: readSqliteCommitStageBeginOk(bc) }
        case 1:
            return { tag: "SqliteFenceMismatch", val: readSqliteFenceMismatch(bc) }
        case 2:
            return { tag: "SqliteErrorResponse", val: readSqliteErrorResponse(bc) }
        default: {
            bc.offset = offset
            throw new bare.BareError(offset, "invalid tag")
        }
    }
}

export function writeSqliteCommitStageBeginResponse(bc: bare.ByteCursor, x: SqliteCommitStageBeginResponse): void {
    switch (x.tag) {
        case "SqliteCommitStageBeginOk": {
            bare.writeU8(bc, 0)
            writeSqliteCommitStageBeginOk(bc, x.val)
            break
        }
        case "SqliteFenceMismatch": {
            bare.writeU8(bc, 1)
            writeSqliteFenceMismatch(bc, x.val)
            break
        }
        case "SqliteErrorResponse": {
            bare.writeU8(bc, 2)
            writeSqliteErrorResponse(bc, x.val)
            break
        }
    }
}

export type SqliteCommitStageRequest = {
    readonly actorId: Id
    readonly generation: SqliteGeneration
    readonly txid: SqliteTxid
    readonly chunkIdx: u32
    readonly bytes: ArrayBuffer
    readonly isLast: boolean
}

export function readSqliteCommitStageRequest(bc: bare.ByteCursor): SqliteCommitStageRequest {
    return {
        actorId: readId(bc),
        generation: readSqliteGeneration(bc),
        txid: readSqliteTxid(bc),
        chunkIdx: bare.readU32(bc),
        bytes: bare.readData(bc),
        isLast: bare.readBool(bc),
    }
}

export function writeSqliteCommitStageRequest(bc: bare.ByteCursor, x: SqliteCommitStageRequest): void {
    writeId(bc, x.actorId)
    writeSqliteGeneration(bc, x.generation)
    writeSqliteTxid(bc, x.txid)
    bare.writeU32(bc, x.chunkIdx)
    bare.writeData(bc, x.bytes)
    bare.writeBool(bc, x.isLast)
}

export type SqliteCommitStageOk = {
    readonly chunkIdxCommitted: u32
}

export function readSqliteCommitStageOk(bc: bare.ByteCursor): SqliteCommitStageOk {
    return {
        chunkIdxCommitted: bare.readU32(bc),
    }
}

export function writeSqliteCommitStageOk(bc: bare.ByteCursor, x: SqliteCommitStageOk): void {
    bare.writeU32(bc, x.chunkIdxCommitted)
}

export type SqliteCommitStageResponse =
    | { readonly tag: "SqliteCommitStageOk"; readonly val: SqliteCommitStageOk }
    | { readonly tag: "SqliteFenceMismatch"; readonly val: SqliteFenceMismatch }
    | { readonly tag: "SqliteErrorResponse"; readonly val: SqliteErrorResponse }

export function readSqliteCommitStageResponse(bc: bare.ByteCursor): SqliteCommitStageResponse {
    const offset = bc.offset
    const tag = bare.readU8(bc)
    switch (tag) {
        case 0:
            return { tag: "SqliteCommitStageOk", val: readSqliteCommitStageOk(bc) }
        case 1:
            return { tag: "SqliteFenceMismatch", val: readSqliteFenceMismatch(bc) }
        case 2:
            return { tag: "SqliteErrorResponse", val: readSqliteErrorResponse(bc) }
        default: {
            bc.offset = offset
            throw new bare.BareError(offset, "invalid tag")
        }
    }
}

export function writeSqliteCommitStageResponse(bc: bare.ByteCursor, x: SqliteCommitStageResponse): void {
    switch (x.tag) {
        case "SqliteCommitStageOk": {
            bare.writeU8(bc, 0)
            writeSqliteCommitStageOk(bc, x.val)
            break
        }
        case "SqliteFenceMismatch": {
            bare.writeU8(bc, 1)
            writeSqliteFenceMismatch(bc, x.val)
            break
        }
        case "SqliteErrorResponse": {
            bare.writeU8(bc, 2)
            writeSqliteErrorResponse(bc, x.val)
            break
        }
    }
}

export type SqliteCommitFinalizeRequest = {
    readonly actorId: Id
    readonly generation: SqliteGeneration
    readonly expectedHeadTxid: SqliteTxid
    readonly txid: SqliteTxid
    readonly newDbSizePages: u32
}

export function readSqliteCommitFinalizeRequest(bc: bare.ByteCursor): SqliteCommitFinalizeRequest {
    return {
        actorId: readId(bc),
        generation: readSqliteGeneration(bc),
        expectedHeadTxid: readSqliteTxid(bc),
        txid: readSqliteTxid(bc),
        newDbSizePages: bare.readU32(bc),
    }
}

export function writeSqliteCommitFinalizeRequest(bc: bare.ByteCursor, x: SqliteCommitFinalizeRequest): void {
    writeId(bc, x.actorId)
    writeSqliteGeneration(bc, x.generation)
    writeSqliteTxid(bc, x.expectedHeadTxid)
    writeSqliteTxid(bc, x.txid)
    bare.writeU32(bc, x.newDbSizePages)
}

export type SqliteCommitFinalizeOk = {
    readonly newHeadTxid: SqliteTxid
    readonly meta: SqliteMeta
}

export function readSqliteCommitFinalizeOk(bc: bare.ByteCursor): SqliteCommitFinalizeOk {
    return {
        newHeadTxid: readSqliteTxid(bc),
        meta: readSqliteMeta(bc),
    }
}

export function writeSqliteCommitFinalizeOk(bc: bare.ByteCursor, x: SqliteCommitFinalizeOk): void {
    writeSqliteTxid(bc, x.newHeadTxid)
    writeSqliteMeta(bc, x.meta)
}

export type SqliteStageNotFound = {
    readonly stageId: SqliteStageId
}

export function readSqliteStageNotFound(bc: bare.ByteCursor): SqliteStageNotFound {
    return {
        stageId: readSqliteStageId(bc),
    }
}

export function writeSqliteStageNotFound(bc: bare.ByteCursor, x: SqliteStageNotFound): void {
    writeSqliteStageId(bc, x.stageId)
}

export type SqliteCommitFinalizeResponse =
    | { readonly tag: "SqliteCommitFinalizeOk"; readonly val: SqliteCommitFinalizeOk }
    | { readonly tag: "SqliteFenceMismatch"; readonly val: SqliteFenceMismatch }
    | { readonly tag: "SqliteStageNotFound"; readonly val: SqliteStageNotFound }
    | { readonly tag: "SqliteErrorResponse"; readonly val: SqliteErrorResponse }

export function readSqliteCommitFinalizeResponse(bc: bare.ByteCursor): SqliteCommitFinalizeResponse {
    const offset = bc.offset
    const tag = bare.readU8(bc)
    switch (tag) {
        case 0:
            return { tag: "SqliteCommitFinalizeOk", val: readSqliteCommitFinalizeOk(bc) }
        case 1:
            return { tag: "SqliteFenceMismatch", val: readSqliteFenceMismatch(bc) }
        case 2:
            return { tag: "SqliteStageNotFound", val: readSqliteStageNotFound(bc) }
        case 3:
            return { tag: "SqliteErrorResponse", val: readSqliteErrorResponse(bc) }
        default: {
            bc.offset = offset
            throw new bare.BareError(offset, "invalid tag")
        }
    }
}

export function writeSqliteCommitFinalizeResponse(bc: bare.ByteCursor, x: SqliteCommitFinalizeResponse): void {
    switch (x.tag) {
        case "SqliteCommitFinalizeOk": {
            bare.writeU8(bc, 0)
            writeSqliteCommitFinalizeOk(bc, x.val)
            break
        }
        case "SqliteFenceMismatch": {
            bare.writeU8(bc, 1)
            writeSqliteFenceMismatch(bc, x.val)
            break
        }
        case "SqliteStageNotFound": {
            bare.writeU8(bc, 2)
            writeSqliteStageNotFound(bc, x.val)
            break
        }
        case "SqliteErrorResponse": {
            bare.writeU8(bc, 3)
            writeSqliteErrorResponse(bc, x.val)
            break
        }
    }
}

export type SqliteStartupData = {
    readonly generation: SqliteGeneration
    readonly meta: SqliteMeta
    readonly preloadedPages: readonly SqliteFetchedPage[]
}

export function readSqliteStartupData(bc: bare.ByteCursor): SqliteStartupData {
    return {
        generation: readSqliteGeneration(bc),
        meta: readSqliteMeta(bc),
        preloadedPages: read7(bc),
    }
}

export function writeSqliteStartupData(bc: bare.ByteCursor, x: SqliteStartupData): void {
    writeSqliteGeneration(bc, x.generation)
    writeSqliteMeta(bc, x.meta)
    write7(bc, x.preloadedPages)
}

/**
 * Core
 */
export enum StopCode {
    Ok = "Ok",
    Error = "Error",
}

export function readStopCode(bc: bare.ByteCursor): StopCode {
    const offset = bc.offset
    const tag = bare.readU8(bc)
    switch (tag) {
        case 0:
            return StopCode.Ok
        case 1:
            return StopCode.Error
        default: {
            bc.offset = offset
            throw new bare.BareError(offset, "invalid tag")
        }
    }
}

export function writeStopCode(bc: bare.ByteCursor, x: StopCode): void {
    switch (x) {
        case StopCode.Ok: {
            bare.writeU8(bc, 0)
            break
        }
        case StopCode.Error: {
            bare.writeU8(bc, 1)
            break
        }
    }
}

export type ActorName = {
    readonly metadata: Json
}

export function readActorName(bc: bare.ByteCursor): ActorName {
    return {
        metadata: readJson(bc),
    }
}

export function writeActorName(bc: bare.ByteCursor, x: ActorName): void {
    writeJson(bc, x.metadata)
}

function read9(bc: bare.ByteCursor): string | null {
    return bare.readBool(bc) ? bare.readString(bc) : null
}

function write9(bc: bare.ByteCursor, x: string | null): void {
    bare.writeBool(bc, x != null)
    if (x != null) {
        bare.writeString(bc, x)
    }
}

function read10(bc: bare.ByteCursor): ArrayBuffer | null {
    return bare.readBool(bc) ? bare.readData(bc) : null
}

function write10(bc: bare.ByteCursor, x: ArrayBuffer | null): void {
    bare.writeBool(bc, x != null)
    if (x != null) {
        bare.writeData(bc, x)
    }
}

export type ActorConfig = {
    readonly name: string
    readonly key: string | null
    readonly createTs: i64
    readonly input: ArrayBuffer | null
}

export function readActorConfig(bc: bare.ByteCursor): ActorConfig {
    return {
        name: bare.readString(bc),
        key: read9(bc),
        createTs: bare.readI64(bc),
        input: read10(bc),
    }
}

export function writeActorConfig(bc: bare.ByteCursor, x: ActorConfig): void {
    bare.writeString(bc, x.name)
    write9(bc, x.key)
    bare.writeI64(bc, x.createTs)
    write10(bc, x.input)
}

export type ActorCheckpoint = {
    readonly actorId: Id
    readonly generation: u32
    readonly index: i64
}

export function readActorCheckpoint(bc: bare.ByteCursor): ActorCheckpoint {
    return {
        actorId: readId(bc),
        generation: bare.readU32(bc),
        index: bare.readI64(bc),
    }
}

export function writeActorCheckpoint(bc: bare.ByteCursor, x: ActorCheckpoint): void {
    writeId(bc, x.actorId)
    bare.writeU32(bc, x.generation)
    bare.writeI64(bc, x.index)
}

/**
 * Intent
 */
export type ActorIntentSleep = null

export type ActorIntentStop = null

export type ActorIntent =
    | { readonly tag: "ActorIntentSleep"; readonly val: ActorIntentSleep }
    | { readonly tag: "ActorIntentStop"; readonly val: ActorIntentStop }

export function readActorIntent(bc: bare.ByteCursor): ActorIntent {
    const offset = bc.offset
    const tag = bare.readU8(bc)
    switch (tag) {
        case 0:
            return { tag: "ActorIntentSleep", val: null }
        case 1:
            return { tag: "ActorIntentStop", val: null }
        default: {
            bc.offset = offset
            throw new bare.BareError(offset, "invalid tag")
        }
    }
}

export function writeActorIntent(bc: bare.ByteCursor, x: ActorIntent): void {
    switch (x.tag) {
        case "ActorIntentSleep": {
            bare.writeU8(bc, 0)
            break
        }
        case "ActorIntentStop": {
            bare.writeU8(bc, 1)
            break
        }
    }
}

/**
 * State
 */
export type ActorStateRunning = null

export type ActorStateStopped = {
    readonly code: StopCode
    readonly message: string | null
}

export function readActorStateStopped(bc: bare.ByteCursor): ActorStateStopped {
    return {
        code: readStopCode(bc),
        message: read9(bc),
    }
}

export function writeActorStateStopped(bc: bare.ByteCursor, x: ActorStateStopped): void {
    writeStopCode(bc, x.code)
    write9(bc, x.message)
}

export type ActorState =
    | { readonly tag: "ActorStateRunning"; readonly val: ActorStateRunning }
    | { readonly tag: "ActorStateStopped"; readonly val: ActorStateStopped }

export function readActorState(bc: bare.ByteCursor): ActorState {
    const offset = bc.offset
    const tag = bare.readU8(bc)
    switch (tag) {
        case 0:
            return { tag: "ActorStateRunning", val: null }
        case 1:
            return { tag: "ActorStateStopped", val: readActorStateStopped(bc) }
        default: {
            bc.offset = offset
            throw new bare.BareError(offset, "invalid tag")
        }
    }
}

export function writeActorState(bc: bare.ByteCursor, x: ActorState): void {
    switch (x.tag) {
        case "ActorStateRunning": {
            bare.writeU8(bc, 0)
            break
        }
        case "ActorStateStopped": {
            bare.writeU8(bc, 1)
            writeActorStateStopped(bc, x.val)
            break
        }
    }
}

/**
 * MARK: Events
 */
export type EventActorIntent = {
    readonly intent: ActorIntent
}

export function readEventActorIntent(bc: bare.ByteCursor): EventActorIntent {
    return {
        intent: readActorIntent(bc),
    }
}

export function writeEventActorIntent(bc: bare.ByteCursor, x: EventActorIntent): void {
    writeActorIntent(bc, x.intent)
}

export type EventActorStateUpdate = {
    readonly state: ActorState
}

export function readEventActorStateUpdate(bc: bare.ByteCursor): EventActorStateUpdate {
    return {
        state: readActorState(bc),
    }
}

export function writeEventActorStateUpdate(bc: bare.ByteCursor, x: EventActorStateUpdate): void {
    writeActorState(bc, x.state)
}

function read11(bc: bare.ByteCursor): i64 | null {
    return bare.readBool(bc) ? bare.readI64(bc) : null
}

function write11(bc: bare.ByteCursor, x: i64 | null): void {
    bare.writeBool(bc, x != null)
    if (x != null) {
        bare.writeI64(bc, x)
    }
}

export type EventActorSetAlarm = {
    readonly alarmTs: i64 | null
}

export function readEventActorSetAlarm(bc: bare.ByteCursor): EventActorSetAlarm {
    return {
        alarmTs: read11(bc),
    }
}

export function writeEventActorSetAlarm(bc: bare.ByteCursor, x: EventActorSetAlarm): void {
    write11(bc, x.alarmTs)
}

export type Event =
    | { readonly tag: "EventActorIntent"; readonly val: EventActorIntent }
    | { readonly tag: "EventActorStateUpdate"; readonly val: EventActorStateUpdate }
    | { readonly tag: "EventActorSetAlarm"; readonly val: EventActorSetAlarm }

export function readEvent(bc: bare.ByteCursor): Event {
    const offset = bc.offset
    const tag = bare.readU8(bc)
    switch (tag) {
        case 0:
            return { tag: "EventActorIntent", val: readEventActorIntent(bc) }
        case 1:
            return { tag: "EventActorStateUpdate", val: readEventActorStateUpdate(bc) }
        case 2:
            return { tag: "EventActorSetAlarm", val: readEventActorSetAlarm(bc) }
        default: {
            bc.offset = offset
            throw new bare.BareError(offset, "invalid tag")
        }
    }
}

export function writeEvent(bc: bare.ByteCursor, x: Event): void {
    switch (x.tag) {
        case "EventActorIntent": {
            bare.writeU8(bc, 0)
            writeEventActorIntent(bc, x.val)
            break
        }
        case "EventActorStateUpdate": {
            bare.writeU8(bc, 1)
            writeEventActorStateUpdate(bc, x.val)
            break
        }
        case "EventActorSetAlarm": {
            bare.writeU8(bc, 2)
            writeEventActorSetAlarm(bc, x.val)
            break
        }
    }
}

export type EventWrapper = {
    readonly checkpoint: ActorCheckpoint
    readonly inner: Event
}

export function readEventWrapper(bc: bare.ByteCursor): EventWrapper {
    return {
        checkpoint: readActorCheckpoint(bc),
        inner: readEvent(bc),
    }
}

export function writeEventWrapper(bc: bare.ByteCursor, x: EventWrapper): void {
    writeActorCheckpoint(bc, x.checkpoint)
    writeEvent(bc, x.inner)
}

export type PreloadedKvEntry = {
    readonly key: KvKey
    readonly value: KvValue
    readonly metadata: KvMetadata
}

export function readPreloadedKvEntry(bc: bare.ByteCursor): PreloadedKvEntry {
    return {
        key: readKvKey(bc),
        value: readKvValue(bc),
        metadata: readKvMetadata(bc),
    }
}

export function writePreloadedKvEntry(bc: bare.ByteCursor, x: PreloadedKvEntry): void {
    writeKvKey(bc, x.key)
    writeKvValue(bc, x.value)
    writeKvMetadata(bc, x.metadata)
}

function read12(bc: bare.ByteCursor): readonly PreloadedKvEntry[] {
    const len = bare.readUintSafe(bc)
    if (len === 0) {
        return []
    }
    const result = [readPreloadedKvEntry(bc)]
    for (let i = 1; i < len; i++) {
        result[i] = readPreloadedKvEntry(bc)
    }
    return result
}

function write12(bc: bare.ByteCursor, x: readonly PreloadedKvEntry[]): void {
    bare.writeUintSafe(bc, x.length)
    for (let i = 0; i < x.length; i++) {
        writePreloadedKvEntry(bc, x[i])
    }
}

export type PreloadedKv = {
    readonly entries: readonly PreloadedKvEntry[]
    readonly requestedGetKeys: readonly KvKey[]
    readonly requestedPrefixes: readonly KvKey[]
}

export function readPreloadedKv(bc: bare.ByteCursor): PreloadedKv {
    return {
        entries: read12(bc),
        requestedGetKeys: read0(bc),
        requestedPrefixes: read0(bc),
    }
}

export function writePreloadedKv(bc: bare.ByteCursor, x: PreloadedKv): void {
    write12(bc, x.entries)
    write0(bc, x.requestedGetKeys)
    write0(bc, x.requestedPrefixes)
}

export type HibernatingRequest = {
    readonly gatewayId: GatewayId
    readonly requestId: RequestId
}

export function readHibernatingRequest(bc: bare.ByteCursor): HibernatingRequest {
    return {
        gatewayId: readGatewayId(bc),
        requestId: readRequestId(bc),
    }
}

export function writeHibernatingRequest(bc: bare.ByteCursor, x: HibernatingRequest): void {
    writeGatewayId(bc, x.gatewayId)
    writeRequestId(bc, x.requestId)
}

function read13(bc: bare.ByteCursor): readonly HibernatingRequest[] {
    const len = bare.readUintSafe(bc)
    if (len === 0) {
        return []
    }
    const result = [readHibernatingRequest(bc)]
    for (let i = 1; i < len; i++) {
        result[i] = readHibernatingRequest(bc)
    }
    return result
}

function write13(bc: bare.ByteCursor, x: readonly HibernatingRequest[]): void {
    bare.writeUintSafe(bc, x.length)
    for (let i = 0; i < x.length; i++) {
        writeHibernatingRequest(bc, x[i])
    }
}

function read14(bc: bare.ByteCursor): PreloadedKv | null {
    return bare.readBool(bc) ? readPreloadedKv(bc) : null
}

function write14(bc: bare.ByteCursor, x: PreloadedKv | null): void {
    bare.writeBool(bc, x != null)
    if (x != null) {
        writePreloadedKv(bc, x)
    }
}

function read15(bc: bare.ByteCursor): SqliteStartupData | null {
    return bare.readBool(bc) ? readSqliteStartupData(bc) : null
}

function write15(bc: bare.ByteCursor, x: SqliteStartupData | null): void {
    bare.writeBool(bc, x != null)
    if (x != null) {
        writeSqliteStartupData(bc, x)
    }
}

export type CommandStartActor = {
    readonly config: ActorConfig
    readonly hibernatingRequests: readonly HibernatingRequest[]
    readonly preloadedKv: PreloadedKv | null
    readonly sqliteSchemaVersion: u32
    readonly sqliteStartupData: SqliteStartupData | null
}

export function readCommandStartActor(bc: bare.ByteCursor): CommandStartActor {
    return {
        config: readActorConfig(bc),
        hibernatingRequests: read13(bc),
        preloadedKv: read14(bc),
        sqliteSchemaVersion: bare.readU32(bc),
        sqliteStartupData: read15(bc),
    }
}

export function writeCommandStartActor(bc: bare.ByteCursor, x: CommandStartActor): void {
    writeActorConfig(bc, x.config)
    write13(bc, x.hibernatingRequests)
    write14(bc, x.preloadedKv)
    bare.writeU32(bc, x.sqliteSchemaVersion)
    write15(bc, x.sqliteStartupData)
}

export enum StopActorReason {
    SleepIntent = "SleepIntent",
    StopIntent = "StopIntent",
    Destroy = "Destroy",
    GoingAway = "GoingAway",
    Lost = "Lost",
}

export function readStopActorReason(bc: bare.ByteCursor): StopActorReason {
    const offset = bc.offset
    const tag = bare.readU8(bc)
    switch (tag) {
        case 0:
            return StopActorReason.SleepIntent
        case 1:
            return StopActorReason.StopIntent
        case 2:
            return StopActorReason.Destroy
        case 3:
            return StopActorReason.GoingAway
        case 4:
            return StopActorReason.Lost
        default: {
            bc.offset = offset
            throw new bare.BareError(offset, "invalid tag")
        }
    }
}

export function writeStopActorReason(bc: bare.ByteCursor, x: StopActorReason): void {
    switch (x) {
        case StopActorReason.SleepIntent: {
            bare.writeU8(bc, 0)
            break
        }
        case StopActorReason.StopIntent: {
            bare.writeU8(bc, 1)
            break
        }
        case StopActorReason.Destroy: {
            bare.writeU8(bc, 2)
            break
        }
        case StopActorReason.GoingAway: {
            bare.writeU8(bc, 3)
            break
        }
        case StopActorReason.Lost: {
            bare.writeU8(bc, 4)
            break
        }
    }
}

export type CommandStopActor = {
    readonly reason: StopActorReason
}

export function readCommandStopActor(bc: bare.ByteCursor): CommandStopActor {
    return {
        reason: readStopActorReason(bc),
    }
}

export function writeCommandStopActor(bc: bare.ByteCursor, x: CommandStopActor): void {
    writeStopActorReason(bc, x.reason)
}

export type Command =
    | { readonly tag: "CommandStartActor"; readonly val: CommandStartActor }
    | { readonly tag: "CommandStopActor"; readonly val: CommandStopActor }

export function readCommand(bc: bare.ByteCursor): Command {
    const offset = bc.offset
    const tag = bare.readU8(bc)
    switch (tag) {
        case 0:
            return { tag: "CommandStartActor", val: readCommandStartActor(bc) }
        case 1:
            return { tag: "CommandStopActor", val: readCommandStopActor(bc) }
        default: {
            bc.offset = offset
            throw new bare.BareError(offset, "invalid tag")
        }
    }
}

export function writeCommand(bc: bare.ByteCursor, x: Command): void {
    switch (x.tag) {
        case "CommandStartActor": {
            bare.writeU8(bc, 0)
            writeCommandStartActor(bc, x.val)
            break
        }
        case "CommandStopActor": {
            bare.writeU8(bc, 1)
            writeCommandStopActor(bc, x.val)
            break
        }
    }
}

export type CommandWrapper = {
    readonly checkpoint: ActorCheckpoint
    readonly inner: Command
}

export function readCommandWrapper(bc: bare.ByteCursor): CommandWrapper {
    return {
        checkpoint: readActorCheckpoint(bc),
        inner: readCommand(bc),
    }
}

export function writeCommandWrapper(bc: bare.ByteCursor, x: CommandWrapper): void {
    writeActorCheckpoint(bc, x.checkpoint)
    writeCommand(bc, x.inner)
}

/**
 * We redeclare this so its top level
 */
export type ActorCommandKeyData =
    | { readonly tag: "CommandStartActor"; readonly val: CommandStartActor }
    | { readonly tag: "CommandStopActor"; readonly val: CommandStopActor }

export function readActorCommandKeyData(bc: bare.ByteCursor): ActorCommandKeyData {
    const offset = bc.offset
    const tag = bare.readU8(bc)
    switch (tag) {
        case 0:
            return { tag: "CommandStartActor", val: readCommandStartActor(bc) }
        case 1:
            return { tag: "CommandStopActor", val: readCommandStopActor(bc) }
        default: {
            bc.offset = offset
            throw new bare.BareError(offset, "invalid tag")
        }
    }
}

export function writeActorCommandKeyData(bc: bare.ByteCursor, x: ActorCommandKeyData): void {
    switch (x.tag) {
        case "CommandStartActor": {
            bare.writeU8(bc, 0)
            writeCommandStartActor(bc, x.val)
            break
        }
        case "CommandStopActor": {
            bare.writeU8(bc, 1)
            writeCommandStopActor(bc, x.val)
            break
        }
    }
}

export function encodeActorCommandKeyData(x: ActorCommandKeyData, config?: Partial<bare.Config>): Uint8Array {
    const fullConfig = config != null ? bare.Config(config) : DEFAULT_CONFIG
    const bc = new bare.ByteCursor(
        new Uint8Array(fullConfig.initialBufferLength),
        fullConfig,
    )
    writeActorCommandKeyData(bc, x)
    return new Uint8Array(bc.view.buffer, bc.view.byteOffset, bc.offset)
}

export function decodeActorCommandKeyData(bytes: Uint8Array): ActorCommandKeyData {
    const bc = new bare.ByteCursor(bytes, DEFAULT_CONFIG)
    const result = readActorCommandKeyData(bc)
    if (bc.offset < bc.view.byteLength) {
        throw new bare.BareError(bc.offset, "remaining bytes")
    }
    return result
}

export type MessageId = {
    /**
     * Globally unique ID
     */
    readonly gatewayId: GatewayId
    /**
     * Unique ID to the gateway
     */
    readonly requestId: RequestId
    /**
     * Unique ID to the request
     */
    readonly messageIndex: MessageIndex
}

export function readMessageId(bc: bare.ByteCursor): MessageId {
    return {
        gatewayId: readGatewayId(bc),
        requestId: readRequestId(bc),
        messageIndex: readMessageIndex(bc),
    }
}

export function writeMessageId(bc: bare.ByteCursor, x: MessageId): void {
    writeGatewayId(bc, x.gatewayId)
    writeRequestId(bc, x.requestId)
    writeMessageIndex(bc, x.messageIndex)
}

function read16(bc: bare.ByteCursor): ReadonlyMap<string, string> {
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

function write16(bc: bare.ByteCursor, x: ReadonlyMap<string, string>): void {
    bare.writeUintSafe(bc, x.size)
    for (const kv of x) {
        bare.writeString(bc, kv[0])
        bare.writeString(bc, kv[1])
    }
}

/**
 * HTTP
 */
export type ToEnvoyRequestStart = {
    readonly actorId: Id
    readonly method: string
    readonly path: string
    readonly headers: ReadonlyMap<string, string>
    readonly body: ArrayBuffer | null
    readonly stream: boolean
}

export function readToEnvoyRequestStart(bc: bare.ByteCursor): ToEnvoyRequestStart {
    return {
        actorId: readId(bc),
        method: bare.readString(bc),
        path: bare.readString(bc),
        headers: read16(bc),
        body: read10(bc),
        stream: bare.readBool(bc),
    }
}

export function writeToEnvoyRequestStart(bc: bare.ByteCursor, x: ToEnvoyRequestStart): void {
    writeId(bc, x.actorId)
    bare.writeString(bc, x.method)
    bare.writeString(bc, x.path)
    write16(bc, x.headers)
    write10(bc, x.body)
    bare.writeBool(bc, x.stream)
}

export type ToEnvoyRequestChunk = {
    readonly body: ArrayBuffer
    readonly finish: boolean
}

export function readToEnvoyRequestChunk(bc: bare.ByteCursor): ToEnvoyRequestChunk {
    return {
        body: bare.readData(bc),
        finish: bare.readBool(bc),
    }
}

export function writeToEnvoyRequestChunk(bc: bare.ByteCursor, x: ToEnvoyRequestChunk): void {
    bare.writeData(bc, x.body)
    bare.writeBool(bc, x.finish)
}

export type ToEnvoyRequestAbort = null

export type ToRivetResponseStart = {
    readonly status: u16
    readonly headers: ReadonlyMap<string, string>
    readonly body: ArrayBuffer | null
    readonly stream: boolean
}

export function readToRivetResponseStart(bc: bare.ByteCursor): ToRivetResponseStart {
    return {
        status: bare.readU16(bc),
        headers: read16(bc),
        body: read10(bc),
        stream: bare.readBool(bc),
    }
}

export function writeToRivetResponseStart(bc: bare.ByteCursor, x: ToRivetResponseStart): void {
    bare.writeU16(bc, x.status)
    write16(bc, x.headers)
    write10(bc, x.body)
    bare.writeBool(bc, x.stream)
}

export type ToRivetResponseChunk = {
    readonly body: ArrayBuffer
    readonly finish: boolean
}

export function readToRivetResponseChunk(bc: bare.ByteCursor): ToRivetResponseChunk {
    return {
        body: bare.readData(bc),
        finish: bare.readBool(bc),
    }
}

export function writeToRivetResponseChunk(bc: bare.ByteCursor, x: ToRivetResponseChunk): void {
    bare.writeData(bc, x.body)
    bare.writeBool(bc, x.finish)
}

export type ToRivetResponseAbort = null

/**
 * WebSocket
 */
export type ToEnvoyWebSocketOpen = {
    readonly actorId: Id
    readonly path: string
    readonly headers: ReadonlyMap<string, string>
}

export function readToEnvoyWebSocketOpen(bc: bare.ByteCursor): ToEnvoyWebSocketOpen {
    return {
        actorId: readId(bc),
        path: bare.readString(bc),
        headers: read16(bc),
    }
}

export function writeToEnvoyWebSocketOpen(bc: bare.ByteCursor, x: ToEnvoyWebSocketOpen): void {
    writeId(bc, x.actorId)
    bare.writeString(bc, x.path)
    write16(bc, x.headers)
}

export type ToEnvoyWebSocketMessage = {
    readonly data: ArrayBuffer
    readonly binary: boolean
}

export function readToEnvoyWebSocketMessage(bc: bare.ByteCursor): ToEnvoyWebSocketMessage {
    return {
        data: bare.readData(bc),
        binary: bare.readBool(bc),
    }
}

export function writeToEnvoyWebSocketMessage(bc: bare.ByteCursor, x: ToEnvoyWebSocketMessage): void {
    bare.writeData(bc, x.data)
    bare.writeBool(bc, x.binary)
}

function read17(bc: bare.ByteCursor): u16 | null {
    return bare.readBool(bc) ? bare.readU16(bc) : null
}

function write17(bc: bare.ByteCursor, x: u16 | null): void {
    bare.writeBool(bc, x != null)
    if (x != null) {
        bare.writeU16(bc, x)
    }
}

export type ToEnvoyWebSocketClose = {
    readonly code: u16 | null
    readonly reason: string | null
}

export function readToEnvoyWebSocketClose(bc: bare.ByteCursor): ToEnvoyWebSocketClose {
    return {
        code: read17(bc),
        reason: read9(bc),
    }
}

export function writeToEnvoyWebSocketClose(bc: bare.ByteCursor, x: ToEnvoyWebSocketClose): void {
    write17(bc, x.code)
    write9(bc, x.reason)
}

export type ToRivetWebSocketOpen = {
    readonly canHibernate: boolean
}

export function readToRivetWebSocketOpen(bc: bare.ByteCursor): ToRivetWebSocketOpen {
    return {
        canHibernate: bare.readBool(bc),
    }
}

export function writeToRivetWebSocketOpen(bc: bare.ByteCursor, x: ToRivetWebSocketOpen): void {
    bare.writeBool(bc, x.canHibernate)
}

export type ToRivetWebSocketMessage = {
    readonly data: ArrayBuffer
    readonly binary: boolean
}

export function readToRivetWebSocketMessage(bc: bare.ByteCursor): ToRivetWebSocketMessage {
    return {
        data: bare.readData(bc),
        binary: bare.readBool(bc),
    }
}

export function writeToRivetWebSocketMessage(bc: bare.ByteCursor, x: ToRivetWebSocketMessage): void {
    bare.writeData(bc, x.data)
    bare.writeBool(bc, x.binary)
}

export type ToRivetWebSocketMessageAck = {
    readonly index: MessageIndex
}

export function readToRivetWebSocketMessageAck(bc: bare.ByteCursor): ToRivetWebSocketMessageAck {
    return {
        index: readMessageIndex(bc),
    }
}

export function writeToRivetWebSocketMessageAck(bc: bare.ByteCursor, x: ToRivetWebSocketMessageAck): void {
    writeMessageIndex(bc, x.index)
}

export type ToRivetWebSocketClose = {
    readonly code: u16 | null
    readonly reason: string | null
    readonly hibernate: boolean
}

export function readToRivetWebSocketClose(bc: bare.ByteCursor): ToRivetWebSocketClose {
    return {
        code: read17(bc),
        reason: read9(bc),
        hibernate: bare.readBool(bc),
    }
}

export function writeToRivetWebSocketClose(bc: bare.ByteCursor, x: ToRivetWebSocketClose): void {
    write17(bc, x.code)
    write9(bc, x.reason)
    bare.writeBool(bc, x.hibernate)
}

/**
 * To Rivet
 */
export type ToRivetTunnelMessageKind =
    /**
     * HTTP
     */
    | { readonly tag: "ToRivetResponseStart"; readonly val: ToRivetResponseStart }
    | { readonly tag: "ToRivetResponseChunk"; readonly val: ToRivetResponseChunk }
    | { readonly tag: "ToRivetResponseAbort"; readonly val: ToRivetResponseAbort }
    /**
     * WebSocket
     */
    | { readonly tag: "ToRivetWebSocketOpen"; readonly val: ToRivetWebSocketOpen }
    | { readonly tag: "ToRivetWebSocketMessage"; readonly val: ToRivetWebSocketMessage }
    | { readonly tag: "ToRivetWebSocketMessageAck"; readonly val: ToRivetWebSocketMessageAck }
    | { readonly tag: "ToRivetWebSocketClose"; readonly val: ToRivetWebSocketClose }

export function readToRivetTunnelMessageKind(bc: bare.ByteCursor): ToRivetTunnelMessageKind {
    const offset = bc.offset
    const tag = bare.readU8(bc)
    switch (tag) {
        case 0:
            return { tag: "ToRivetResponseStart", val: readToRivetResponseStart(bc) }
        case 1:
            return { tag: "ToRivetResponseChunk", val: readToRivetResponseChunk(bc) }
        case 2:
            return { tag: "ToRivetResponseAbort", val: null }
        case 3:
            return { tag: "ToRivetWebSocketOpen", val: readToRivetWebSocketOpen(bc) }
        case 4:
            return { tag: "ToRivetWebSocketMessage", val: readToRivetWebSocketMessage(bc) }
        case 5:
            return { tag: "ToRivetWebSocketMessageAck", val: readToRivetWebSocketMessageAck(bc) }
        case 6:
            return { tag: "ToRivetWebSocketClose", val: readToRivetWebSocketClose(bc) }
        default: {
            bc.offset = offset
            throw new bare.BareError(offset, "invalid tag")
        }
    }
}

export function writeToRivetTunnelMessageKind(bc: bare.ByteCursor, x: ToRivetTunnelMessageKind): void {
    switch (x.tag) {
        case "ToRivetResponseStart": {
            bare.writeU8(bc, 0)
            writeToRivetResponseStart(bc, x.val)
            break
        }
        case "ToRivetResponseChunk": {
            bare.writeU8(bc, 1)
            writeToRivetResponseChunk(bc, x.val)
            break
        }
        case "ToRivetResponseAbort": {
            bare.writeU8(bc, 2)
            break
        }
        case "ToRivetWebSocketOpen": {
            bare.writeU8(bc, 3)
            writeToRivetWebSocketOpen(bc, x.val)
            break
        }
        case "ToRivetWebSocketMessage": {
            bare.writeU8(bc, 4)
            writeToRivetWebSocketMessage(bc, x.val)
            break
        }
        case "ToRivetWebSocketMessageAck": {
            bare.writeU8(bc, 5)
            writeToRivetWebSocketMessageAck(bc, x.val)
            break
        }
        case "ToRivetWebSocketClose": {
            bare.writeU8(bc, 6)
            writeToRivetWebSocketClose(bc, x.val)
            break
        }
    }
}

export type ToRivetTunnelMessage = {
    readonly messageId: MessageId
    readonly messageKind: ToRivetTunnelMessageKind
}

export function readToRivetTunnelMessage(bc: bare.ByteCursor): ToRivetTunnelMessage {
    return {
        messageId: readMessageId(bc),
        messageKind: readToRivetTunnelMessageKind(bc),
    }
}

export function writeToRivetTunnelMessage(bc: bare.ByteCursor, x: ToRivetTunnelMessage): void {
    writeMessageId(bc, x.messageId)
    writeToRivetTunnelMessageKind(bc, x.messageKind)
}

/**
 * To Envoy
 */
export type ToEnvoyTunnelMessageKind =
    /**
     * HTTP
     */
    | { readonly tag: "ToEnvoyRequestStart"; readonly val: ToEnvoyRequestStart }
    | { readonly tag: "ToEnvoyRequestChunk"; readonly val: ToEnvoyRequestChunk }
    | { readonly tag: "ToEnvoyRequestAbort"; readonly val: ToEnvoyRequestAbort }
    /**
     * WebSocket
     */
    | { readonly tag: "ToEnvoyWebSocketOpen"; readonly val: ToEnvoyWebSocketOpen }
    | { readonly tag: "ToEnvoyWebSocketMessage"; readonly val: ToEnvoyWebSocketMessage }
    | { readonly tag: "ToEnvoyWebSocketClose"; readonly val: ToEnvoyWebSocketClose }

export function readToEnvoyTunnelMessageKind(bc: bare.ByteCursor): ToEnvoyTunnelMessageKind {
    const offset = bc.offset
    const tag = bare.readU8(bc)
    switch (tag) {
        case 0:
            return { tag: "ToEnvoyRequestStart", val: readToEnvoyRequestStart(bc) }
        case 1:
            return { tag: "ToEnvoyRequestChunk", val: readToEnvoyRequestChunk(bc) }
        case 2:
            return { tag: "ToEnvoyRequestAbort", val: null }
        case 3:
            return { tag: "ToEnvoyWebSocketOpen", val: readToEnvoyWebSocketOpen(bc) }
        case 4:
            return { tag: "ToEnvoyWebSocketMessage", val: readToEnvoyWebSocketMessage(bc) }
        case 5:
            return { tag: "ToEnvoyWebSocketClose", val: readToEnvoyWebSocketClose(bc) }
        default: {
            bc.offset = offset
            throw new bare.BareError(offset, "invalid tag")
        }
    }
}

export function writeToEnvoyTunnelMessageKind(bc: bare.ByteCursor, x: ToEnvoyTunnelMessageKind): void {
    switch (x.tag) {
        case "ToEnvoyRequestStart": {
            bare.writeU8(bc, 0)
            writeToEnvoyRequestStart(bc, x.val)
            break
        }
        case "ToEnvoyRequestChunk": {
            bare.writeU8(bc, 1)
            writeToEnvoyRequestChunk(bc, x.val)
            break
        }
        case "ToEnvoyRequestAbort": {
            bare.writeU8(bc, 2)
            break
        }
        case "ToEnvoyWebSocketOpen": {
            bare.writeU8(bc, 3)
            writeToEnvoyWebSocketOpen(bc, x.val)
            break
        }
        case "ToEnvoyWebSocketMessage": {
            bare.writeU8(bc, 4)
            writeToEnvoyWebSocketMessage(bc, x.val)
            break
        }
        case "ToEnvoyWebSocketClose": {
            bare.writeU8(bc, 5)
            writeToEnvoyWebSocketClose(bc, x.val)
            break
        }
    }
}

export type ToEnvoyTunnelMessage = {
    readonly messageId: MessageId
    readonly messageKind: ToEnvoyTunnelMessageKind
}

export function readToEnvoyTunnelMessage(bc: bare.ByteCursor): ToEnvoyTunnelMessage {
    return {
        messageId: readMessageId(bc),
        messageKind: readToEnvoyTunnelMessageKind(bc),
    }
}

export function writeToEnvoyTunnelMessage(bc: bare.ByteCursor, x: ToEnvoyTunnelMessage): void {
    writeMessageId(bc, x.messageId)
    writeToEnvoyTunnelMessageKind(bc, x.messageKind)
}

export type ToEnvoyPing = {
    readonly ts: i64
}

export function readToEnvoyPing(bc: bare.ByteCursor): ToEnvoyPing {
    return {
        ts: bare.readI64(bc),
    }
}

export function writeToEnvoyPing(bc: bare.ByteCursor, x: ToEnvoyPing): void {
    bare.writeI64(bc, x.ts)
}

function read18(bc: bare.ByteCursor): ReadonlyMap<string, ActorName> {
    const len = bare.readUintSafe(bc)
    const result = new Map<string, ActorName>()
    for (let i = 0; i < len; i++) {
        const offset = bc.offset
        const key = bare.readString(bc)
        if (result.has(key)) {
            bc.offset = offset
            throw new bare.BareError(offset, "duplicated key")
        }
        result.set(key, readActorName(bc))
    }
    return result
}

function write18(bc: bare.ByteCursor, x: ReadonlyMap<string, ActorName>): void {
    bare.writeUintSafe(bc, x.size)
    for (const kv of x) {
        bare.writeString(bc, kv[0])
        writeActorName(bc, kv[1])
    }
}

function read19(bc: bare.ByteCursor): ReadonlyMap<string, ActorName> | null {
    return bare.readBool(bc) ? read18(bc) : null
}

function write19(bc: bare.ByteCursor, x: ReadonlyMap<string, ActorName> | null): void {
    bare.writeBool(bc, x != null)
    if (x != null) {
        write18(bc, x)
    }
}

function read20(bc: bare.ByteCursor): Json | null {
    return bare.readBool(bc) ? readJson(bc) : null
}

function write20(bc: bare.ByteCursor, x: Json | null): void {
    bare.writeBool(bc, x != null)
    if (x != null) {
        writeJson(bc, x)
    }
}

/**
 * MARK: To Rivet
 */
export type ToRivetMetadata = {
    readonly prepopulateActorNames: ReadonlyMap<string, ActorName> | null
    readonly metadata: Json | null
}

export function readToRivetMetadata(bc: bare.ByteCursor): ToRivetMetadata {
    return {
        prepopulateActorNames: read19(bc),
        metadata: read20(bc),
    }
}

export function writeToRivetMetadata(bc: bare.ByteCursor, x: ToRivetMetadata): void {
    write19(bc, x.prepopulateActorNames)
    write20(bc, x.metadata)
}

export type ToRivetEvents = readonly EventWrapper[]

export function readToRivetEvents(bc: bare.ByteCursor): ToRivetEvents {
    const len = bare.readUintSafe(bc)
    if (len === 0) {
        return []
    }
    const result = [readEventWrapper(bc)]
    for (let i = 1; i < len; i++) {
        result[i] = readEventWrapper(bc)
    }
    return result
}

export function writeToRivetEvents(bc: bare.ByteCursor, x: ToRivetEvents): void {
    bare.writeUintSafe(bc, x.length)
    for (let i = 0; i < x.length; i++) {
        writeEventWrapper(bc, x[i])
    }
}

function read21(bc: bare.ByteCursor): readonly ActorCheckpoint[] {
    const len = bare.readUintSafe(bc)
    if (len === 0) {
        return []
    }
    const result = [readActorCheckpoint(bc)]
    for (let i = 1; i < len; i++) {
        result[i] = readActorCheckpoint(bc)
    }
    return result
}

function write21(bc: bare.ByteCursor, x: readonly ActorCheckpoint[]): void {
    bare.writeUintSafe(bc, x.length)
    for (let i = 0; i < x.length; i++) {
        writeActorCheckpoint(bc, x[i])
    }
}

export type ToRivetAckCommands = {
    readonly lastCommandCheckpoints: readonly ActorCheckpoint[]
}

export function readToRivetAckCommands(bc: bare.ByteCursor): ToRivetAckCommands {
    return {
        lastCommandCheckpoints: read21(bc),
    }
}

export function writeToRivetAckCommands(bc: bare.ByteCursor, x: ToRivetAckCommands): void {
    write21(bc, x.lastCommandCheckpoints)
}

export type ToRivetStopping = null

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

export type ToRivetKvRequest = {
    readonly actorId: Id
    readonly requestId: u32
    readonly data: KvRequestData
}

export function readToRivetKvRequest(bc: bare.ByteCursor): ToRivetKvRequest {
    return {
        actorId: readId(bc),
        requestId: bare.readU32(bc),
        data: readKvRequestData(bc),
    }
}

export function writeToRivetKvRequest(bc: bare.ByteCursor, x: ToRivetKvRequest): void {
    writeId(bc, x.actorId)
    bare.writeU32(bc, x.requestId)
    writeKvRequestData(bc, x.data)
}

export type ToRivetSqliteGetPagesRequest = {
    readonly requestId: u32
    readonly data: SqliteGetPagesRequest
}

export function readToRivetSqliteGetPagesRequest(bc: bare.ByteCursor): ToRivetSqliteGetPagesRequest {
    return {
        requestId: bare.readU32(bc),
        data: readSqliteGetPagesRequest(bc),
    }
}

export function writeToRivetSqliteGetPagesRequest(bc: bare.ByteCursor, x: ToRivetSqliteGetPagesRequest): void {
    bare.writeU32(bc, x.requestId)
    writeSqliteGetPagesRequest(bc, x.data)
}

export type ToRivetSqliteCommitRequest = {
    readonly requestId: u32
    readonly data: SqliteCommitRequest
}

export function readToRivetSqliteCommitRequest(bc: bare.ByteCursor): ToRivetSqliteCommitRequest {
    return {
        requestId: bare.readU32(bc),
        data: readSqliteCommitRequest(bc),
    }
}

export function writeToRivetSqliteCommitRequest(bc: bare.ByteCursor, x: ToRivetSqliteCommitRequest): void {
    bare.writeU32(bc, x.requestId)
    writeSqliteCommitRequest(bc, x.data)
}

export type ToRivetSqliteCommitStageBeginRequest = {
    readonly requestId: u32
    readonly data: SqliteCommitStageBeginRequest
}

export function readToRivetSqliteCommitStageBeginRequest(bc: bare.ByteCursor): ToRivetSqliteCommitStageBeginRequest {
    return {
        requestId: bare.readU32(bc),
        data: readSqliteCommitStageBeginRequest(bc),
    }
}

export function writeToRivetSqliteCommitStageBeginRequest(bc: bare.ByteCursor, x: ToRivetSqliteCommitStageBeginRequest): void {
    bare.writeU32(bc, x.requestId)
    writeSqliteCommitStageBeginRequest(bc, x.data)
}

export type ToRivetSqliteCommitStageRequest = {
    readonly requestId: u32
    readonly data: SqliteCommitStageRequest
}

export function readToRivetSqliteCommitStageRequest(bc: bare.ByteCursor): ToRivetSqliteCommitStageRequest {
    return {
        requestId: bare.readU32(bc),
        data: readSqliteCommitStageRequest(bc),
    }
}

export function writeToRivetSqliteCommitStageRequest(bc: bare.ByteCursor, x: ToRivetSqliteCommitStageRequest): void {
    bare.writeU32(bc, x.requestId)
    writeSqliteCommitStageRequest(bc, x.data)
}

export type ToRivetSqliteCommitFinalizeRequest = {
    readonly requestId: u32
    readonly data: SqliteCommitFinalizeRequest
}

export function readToRivetSqliteCommitFinalizeRequest(bc: bare.ByteCursor): ToRivetSqliteCommitFinalizeRequest {
    return {
        requestId: bare.readU32(bc),
        data: readSqliteCommitFinalizeRequest(bc),
    }
}

export function writeToRivetSqliteCommitFinalizeRequest(bc: bare.ByteCursor, x: ToRivetSqliteCommitFinalizeRequest): void {
    bare.writeU32(bc, x.requestId)
    writeSqliteCommitFinalizeRequest(bc, x.data)
}

export type ToRivet =
    | { readonly tag: "ToRivetMetadata"; readonly val: ToRivetMetadata }
    | { readonly tag: "ToRivetEvents"; readonly val: ToRivetEvents }
    | { readonly tag: "ToRivetAckCommands"; readonly val: ToRivetAckCommands }
    | { readonly tag: "ToRivetStopping"; readonly val: ToRivetStopping }
    | { readonly tag: "ToRivetPong"; readonly val: ToRivetPong }
    | { readonly tag: "ToRivetKvRequest"; readonly val: ToRivetKvRequest }
    | { readonly tag: "ToRivetTunnelMessage"; readonly val: ToRivetTunnelMessage }
    | { readonly tag: "ToRivetSqliteGetPagesRequest"; readonly val: ToRivetSqliteGetPagesRequest }
    | { readonly tag: "ToRivetSqliteCommitRequest"; readonly val: ToRivetSqliteCommitRequest }
    | { readonly tag: "ToRivetSqliteCommitStageBeginRequest"; readonly val: ToRivetSqliteCommitStageBeginRequest }
    | { readonly tag: "ToRivetSqliteCommitStageRequest"; readonly val: ToRivetSqliteCommitStageRequest }
    | { readonly tag: "ToRivetSqliteCommitFinalizeRequest"; readonly val: ToRivetSqliteCommitFinalizeRequest }

export function readToRivet(bc: bare.ByteCursor): ToRivet {
    const offset = bc.offset
    const tag = bare.readU8(bc)
    switch (tag) {
        case 0:
            return { tag: "ToRivetMetadata", val: readToRivetMetadata(bc) }
        case 1:
            return { tag: "ToRivetEvents", val: readToRivetEvents(bc) }
        case 2:
            return { tag: "ToRivetAckCommands", val: readToRivetAckCommands(bc) }
        case 3:
            return { tag: "ToRivetStopping", val: null }
        case 4:
            return { tag: "ToRivetPong", val: readToRivetPong(bc) }
        case 5:
            return { tag: "ToRivetKvRequest", val: readToRivetKvRequest(bc) }
        case 6:
            return { tag: "ToRivetTunnelMessage", val: readToRivetTunnelMessage(bc) }
        case 7:
            return { tag: "ToRivetSqliteGetPagesRequest", val: readToRivetSqliteGetPagesRequest(bc) }
        case 8:
            return { tag: "ToRivetSqliteCommitRequest", val: readToRivetSqliteCommitRequest(bc) }
        case 9:
            return { tag: "ToRivetSqliteCommitStageBeginRequest", val: readToRivetSqliteCommitStageBeginRequest(bc) }
        case 10:
            return { tag: "ToRivetSqliteCommitStageRequest", val: readToRivetSqliteCommitStageRequest(bc) }
        case 11:
            return { tag: "ToRivetSqliteCommitFinalizeRequest", val: readToRivetSqliteCommitFinalizeRequest(bc) }
        default: {
            bc.offset = offset
            throw new bare.BareError(offset, "invalid tag")
        }
    }
}

export function writeToRivet(bc: bare.ByteCursor, x: ToRivet): void {
    switch (x.tag) {
        case "ToRivetMetadata": {
            bare.writeU8(bc, 0)
            writeToRivetMetadata(bc, x.val)
            break
        }
        case "ToRivetEvents": {
            bare.writeU8(bc, 1)
            writeToRivetEvents(bc, x.val)
            break
        }
        case "ToRivetAckCommands": {
            bare.writeU8(bc, 2)
            writeToRivetAckCommands(bc, x.val)
            break
        }
        case "ToRivetStopping": {
            bare.writeU8(bc, 3)
            break
        }
        case "ToRivetPong": {
            bare.writeU8(bc, 4)
            writeToRivetPong(bc, x.val)
            break
        }
        case "ToRivetKvRequest": {
            bare.writeU8(bc, 5)
            writeToRivetKvRequest(bc, x.val)
            break
        }
        case "ToRivetTunnelMessage": {
            bare.writeU8(bc, 6)
            writeToRivetTunnelMessage(bc, x.val)
            break
        }
        case "ToRivetSqliteGetPagesRequest": {
            bare.writeU8(bc, 7)
            writeToRivetSqliteGetPagesRequest(bc, x.val)
            break
        }
        case "ToRivetSqliteCommitRequest": {
            bare.writeU8(bc, 8)
            writeToRivetSqliteCommitRequest(bc, x.val)
            break
        }
        case "ToRivetSqliteCommitStageBeginRequest": {
            bare.writeU8(bc, 9)
            writeToRivetSqliteCommitStageBeginRequest(bc, x.val)
            break
        }
        case "ToRivetSqliteCommitStageRequest": {
            bare.writeU8(bc, 10)
            writeToRivetSqliteCommitStageRequest(bc, x.val)
            break
        }
        case "ToRivetSqliteCommitFinalizeRequest": {
            bare.writeU8(bc, 11)
            writeToRivetSqliteCommitFinalizeRequest(bc, x.val)
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

/**
 * MARK: To Envoy
 */
export type ProtocolMetadata = {
    readonly envoyLostThreshold: i64
    readonly actorStopThreshold: i64
    readonly maxResponsePayloadSize: u64
}

export function readProtocolMetadata(bc: bare.ByteCursor): ProtocolMetadata {
    return {
        envoyLostThreshold: bare.readI64(bc),
        actorStopThreshold: bare.readI64(bc),
        maxResponsePayloadSize: bare.readU64(bc),
    }
}

export function writeProtocolMetadata(bc: bare.ByteCursor, x: ProtocolMetadata): void {
    bare.writeI64(bc, x.envoyLostThreshold)
    bare.writeI64(bc, x.actorStopThreshold)
    bare.writeU64(bc, x.maxResponsePayloadSize)
}

export type ToEnvoyInit = {
    readonly metadata: ProtocolMetadata
}

export function readToEnvoyInit(bc: bare.ByteCursor): ToEnvoyInit {
    return {
        metadata: readProtocolMetadata(bc),
    }
}

export function writeToEnvoyInit(bc: bare.ByteCursor, x: ToEnvoyInit): void {
    writeProtocolMetadata(bc, x.metadata)
}

export type ToEnvoyCommands = readonly CommandWrapper[]

export function readToEnvoyCommands(bc: bare.ByteCursor): ToEnvoyCommands {
    const len = bare.readUintSafe(bc)
    if (len === 0) {
        return []
    }
    const result = [readCommandWrapper(bc)]
    for (let i = 1; i < len; i++) {
        result[i] = readCommandWrapper(bc)
    }
    return result
}

export function writeToEnvoyCommands(bc: bare.ByteCursor, x: ToEnvoyCommands): void {
    bare.writeUintSafe(bc, x.length)
    for (let i = 0; i < x.length; i++) {
        writeCommandWrapper(bc, x[i])
    }
}

export type ToEnvoyAckEvents = {
    readonly lastEventCheckpoints: readonly ActorCheckpoint[]
}

export function readToEnvoyAckEvents(bc: bare.ByteCursor): ToEnvoyAckEvents {
    return {
        lastEventCheckpoints: read21(bc),
    }
}

export function writeToEnvoyAckEvents(bc: bare.ByteCursor, x: ToEnvoyAckEvents): void {
    write21(bc, x.lastEventCheckpoints)
}

export type ToEnvoyKvResponse = {
    readonly requestId: u32
    readonly data: KvResponseData
}

export function readToEnvoyKvResponse(bc: bare.ByteCursor): ToEnvoyKvResponse {
    return {
        requestId: bare.readU32(bc),
        data: readKvResponseData(bc),
    }
}

export function writeToEnvoyKvResponse(bc: bare.ByteCursor, x: ToEnvoyKvResponse): void {
    bare.writeU32(bc, x.requestId)
    writeKvResponseData(bc, x.data)
}

export type ToEnvoySqliteGetPagesResponse = {
    readonly requestId: u32
    readonly data: SqliteGetPagesResponse
}

export function readToEnvoySqliteGetPagesResponse(bc: bare.ByteCursor): ToEnvoySqliteGetPagesResponse {
    return {
        requestId: bare.readU32(bc),
        data: readSqliteGetPagesResponse(bc),
    }
}

export function writeToEnvoySqliteGetPagesResponse(bc: bare.ByteCursor, x: ToEnvoySqliteGetPagesResponse): void {
    bare.writeU32(bc, x.requestId)
    writeSqliteGetPagesResponse(bc, x.data)
}

export type ToEnvoySqliteCommitResponse = {
    readonly requestId: u32
    readonly data: SqliteCommitResponse
}

export function readToEnvoySqliteCommitResponse(bc: bare.ByteCursor): ToEnvoySqliteCommitResponse {
    return {
        requestId: bare.readU32(bc),
        data: readSqliteCommitResponse(bc),
    }
}

export function writeToEnvoySqliteCommitResponse(bc: bare.ByteCursor, x: ToEnvoySqliteCommitResponse): void {
    bare.writeU32(bc, x.requestId)
    writeSqliteCommitResponse(bc, x.data)
}

export type ToEnvoySqliteCommitStageBeginResponse = {
    readonly requestId: u32
    readonly data: SqliteCommitStageBeginResponse
}

export function readToEnvoySqliteCommitStageBeginResponse(bc: bare.ByteCursor): ToEnvoySqliteCommitStageBeginResponse {
    return {
        requestId: bare.readU32(bc),
        data: readSqliteCommitStageBeginResponse(bc),
    }
}

export function writeToEnvoySqliteCommitStageBeginResponse(bc: bare.ByteCursor, x: ToEnvoySqliteCommitStageBeginResponse): void {
    bare.writeU32(bc, x.requestId)
    writeSqliteCommitStageBeginResponse(bc, x.data)
}

export type ToEnvoySqliteCommitStageResponse = {
    readonly requestId: u32
    readonly data: SqliteCommitStageResponse
}

export function readToEnvoySqliteCommitStageResponse(bc: bare.ByteCursor): ToEnvoySqliteCommitStageResponse {
    return {
        requestId: bare.readU32(bc),
        data: readSqliteCommitStageResponse(bc),
    }
}

export function writeToEnvoySqliteCommitStageResponse(bc: bare.ByteCursor, x: ToEnvoySqliteCommitStageResponse): void {
    bare.writeU32(bc, x.requestId)
    writeSqliteCommitStageResponse(bc, x.data)
}

export type ToEnvoySqliteCommitFinalizeResponse = {
    readonly requestId: u32
    readonly data: SqliteCommitFinalizeResponse
}

export function readToEnvoySqliteCommitFinalizeResponse(bc: bare.ByteCursor): ToEnvoySqliteCommitFinalizeResponse {
    return {
        requestId: bare.readU32(bc),
        data: readSqliteCommitFinalizeResponse(bc),
    }
}

export function writeToEnvoySqliteCommitFinalizeResponse(bc: bare.ByteCursor, x: ToEnvoySqliteCommitFinalizeResponse): void {
    bare.writeU32(bc, x.requestId)
    writeSqliteCommitFinalizeResponse(bc, x.data)
}

export type ToEnvoy =
    | { readonly tag: "ToEnvoyInit"; readonly val: ToEnvoyInit }
    | { readonly tag: "ToEnvoyCommands"; readonly val: ToEnvoyCommands }
    | { readonly tag: "ToEnvoyAckEvents"; readonly val: ToEnvoyAckEvents }
    | { readonly tag: "ToEnvoyKvResponse"; readonly val: ToEnvoyKvResponse }
    | { readonly tag: "ToEnvoyTunnelMessage"; readonly val: ToEnvoyTunnelMessage }
    | { readonly tag: "ToEnvoyPing"; readonly val: ToEnvoyPing }
    | { readonly tag: "ToEnvoySqliteGetPagesResponse"; readonly val: ToEnvoySqliteGetPagesResponse }
    | { readonly tag: "ToEnvoySqliteCommitResponse"; readonly val: ToEnvoySqliteCommitResponse }
    | { readonly tag: "ToEnvoySqliteCommitStageBeginResponse"; readonly val: ToEnvoySqliteCommitStageBeginResponse }
    | { readonly tag: "ToEnvoySqliteCommitStageResponse"; readonly val: ToEnvoySqliteCommitStageResponse }
    | { readonly tag: "ToEnvoySqliteCommitFinalizeResponse"; readonly val: ToEnvoySqliteCommitFinalizeResponse }

export function readToEnvoy(bc: bare.ByteCursor): ToEnvoy {
    const offset = bc.offset
    const tag = bare.readU8(bc)
    switch (tag) {
        case 0:
            return { tag: "ToEnvoyInit", val: readToEnvoyInit(bc) }
        case 1:
            return { tag: "ToEnvoyCommands", val: readToEnvoyCommands(bc) }
        case 2:
            return { tag: "ToEnvoyAckEvents", val: readToEnvoyAckEvents(bc) }
        case 3:
            return { tag: "ToEnvoyKvResponse", val: readToEnvoyKvResponse(bc) }
        case 4:
            return { tag: "ToEnvoyTunnelMessage", val: readToEnvoyTunnelMessage(bc) }
        case 5:
            return { tag: "ToEnvoyPing", val: readToEnvoyPing(bc) }
        case 6:
            return { tag: "ToEnvoySqliteGetPagesResponse", val: readToEnvoySqliteGetPagesResponse(bc) }
        case 7:
            return { tag: "ToEnvoySqliteCommitResponse", val: readToEnvoySqliteCommitResponse(bc) }
        case 8:
            return { tag: "ToEnvoySqliteCommitStageBeginResponse", val: readToEnvoySqliteCommitStageBeginResponse(bc) }
        case 9:
            return { tag: "ToEnvoySqliteCommitStageResponse", val: readToEnvoySqliteCommitStageResponse(bc) }
        case 10:
            return { tag: "ToEnvoySqliteCommitFinalizeResponse", val: readToEnvoySqliteCommitFinalizeResponse(bc) }
        default: {
            bc.offset = offset
            throw new bare.BareError(offset, "invalid tag")
        }
    }
}

export function writeToEnvoy(bc: bare.ByteCursor, x: ToEnvoy): void {
    switch (x.tag) {
        case "ToEnvoyInit": {
            bare.writeU8(bc, 0)
            writeToEnvoyInit(bc, x.val)
            break
        }
        case "ToEnvoyCommands": {
            bare.writeU8(bc, 1)
            writeToEnvoyCommands(bc, x.val)
            break
        }
        case "ToEnvoyAckEvents": {
            bare.writeU8(bc, 2)
            writeToEnvoyAckEvents(bc, x.val)
            break
        }
        case "ToEnvoyKvResponse": {
            bare.writeU8(bc, 3)
            writeToEnvoyKvResponse(bc, x.val)
            break
        }
        case "ToEnvoyTunnelMessage": {
            bare.writeU8(bc, 4)
            writeToEnvoyTunnelMessage(bc, x.val)
            break
        }
        case "ToEnvoyPing": {
            bare.writeU8(bc, 5)
            writeToEnvoyPing(bc, x.val)
            break
        }
        case "ToEnvoySqliteGetPagesResponse": {
            bare.writeU8(bc, 6)
            writeToEnvoySqliteGetPagesResponse(bc, x.val)
            break
        }
        case "ToEnvoySqliteCommitResponse": {
            bare.writeU8(bc, 7)
            writeToEnvoySqliteCommitResponse(bc, x.val)
            break
        }
        case "ToEnvoySqliteCommitStageBeginResponse": {
            bare.writeU8(bc, 8)
            writeToEnvoySqliteCommitStageBeginResponse(bc, x.val)
            break
        }
        case "ToEnvoySqliteCommitStageResponse": {
            bare.writeU8(bc, 9)
            writeToEnvoySqliteCommitStageResponse(bc, x.val)
            break
        }
        case "ToEnvoySqliteCommitFinalizeResponse": {
            bare.writeU8(bc, 10)
            writeToEnvoySqliteCommitFinalizeResponse(bc, x.val)
            break
        }
    }
}

export function encodeToEnvoy(x: ToEnvoy, config?: Partial<bare.Config>): Uint8Array {
    const fullConfig = config != null ? bare.Config(config) : DEFAULT_CONFIG
    const bc = new bare.ByteCursor(
        new Uint8Array(fullConfig.initialBufferLength),
        fullConfig,
    )
    writeToEnvoy(bc, x)
    return new Uint8Array(bc.view.buffer, bc.view.byteOffset, bc.offset)
}

export function decodeToEnvoy(bytes: Uint8Array): ToEnvoy {
    const bc = new bare.ByteCursor(bytes, DEFAULT_CONFIG)
    const result = readToEnvoy(bc)
    if (bc.offset < bc.view.byteLength) {
        throw new bare.BareError(bc.offset, "remaining bytes")
    }
    return result
}

/**
 * MARK: To Envoy Conn
 */
export type ToEnvoyConnPing = {
    readonly gatewayId: GatewayId
    readonly requestId: RequestId
    readonly ts: i64
}

export function readToEnvoyConnPing(bc: bare.ByteCursor): ToEnvoyConnPing {
    return {
        gatewayId: readGatewayId(bc),
        requestId: readRequestId(bc),
        ts: bare.readI64(bc),
    }
}

export function writeToEnvoyConnPing(bc: bare.ByteCursor, x: ToEnvoyConnPing): void {
    writeGatewayId(bc, x.gatewayId)
    writeRequestId(bc, x.requestId)
    bare.writeI64(bc, x.ts)
}

export type ToEnvoyConnClose = null

export type ToEnvoyConn =
    | { readonly tag: "ToEnvoyConnPing"; readonly val: ToEnvoyConnPing }
    | { readonly tag: "ToEnvoyConnClose"; readonly val: ToEnvoyConnClose }
    | { readonly tag: "ToEnvoyCommands"; readonly val: ToEnvoyCommands }
    | { readonly tag: "ToEnvoyAckEvents"; readonly val: ToEnvoyAckEvents }
    | { readonly tag: "ToEnvoyTunnelMessage"; readonly val: ToEnvoyTunnelMessage }

export function readToEnvoyConn(bc: bare.ByteCursor): ToEnvoyConn {
    const offset = bc.offset
    const tag = bare.readU8(bc)
    switch (tag) {
        case 0:
            return { tag: "ToEnvoyConnPing", val: readToEnvoyConnPing(bc) }
        case 1:
            return { tag: "ToEnvoyConnClose", val: null }
        case 2:
            return { tag: "ToEnvoyCommands", val: readToEnvoyCommands(bc) }
        case 3:
            return { tag: "ToEnvoyAckEvents", val: readToEnvoyAckEvents(bc) }
        case 4:
            return { tag: "ToEnvoyTunnelMessage", val: readToEnvoyTunnelMessage(bc) }
        default: {
            bc.offset = offset
            throw new bare.BareError(offset, "invalid tag")
        }
    }
}

export function writeToEnvoyConn(bc: bare.ByteCursor, x: ToEnvoyConn): void {
    switch (x.tag) {
        case "ToEnvoyConnPing": {
            bare.writeU8(bc, 0)
            writeToEnvoyConnPing(bc, x.val)
            break
        }
        case "ToEnvoyConnClose": {
            bare.writeU8(bc, 1)
            break
        }
        case "ToEnvoyCommands": {
            bare.writeU8(bc, 2)
            writeToEnvoyCommands(bc, x.val)
            break
        }
        case "ToEnvoyAckEvents": {
            bare.writeU8(bc, 3)
            writeToEnvoyAckEvents(bc, x.val)
            break
        }
        case "ToEnvoyTunnelMessage": {
            bare.writeU8(bc, 4)
            writeToEnvoyTunnelMessage(bc, x.val)
            break
        }
    }
}

export function encodeToEnvoyConn(x: ToEnvoyConn, config?: Partial<bare.Config>): Uint8Array {
    const fullConfig = config != null ? bare.Config(config) : DEFAULT_CONFIG
    const bc = new bare.ByteCursor(
        new Uint8Array(fullConfig.initialBufferLength),
        fullConfig,
    )
    writeToEnvoyConn(bc, x)
    return new Uint8Array(bc.view.buffer, bc.view.byteOffset, bc.offset)
}

export function decodeToEnvoyConn(bytes: Uint8Array): ToEnvoyConn {
    const bc = new bare.ByteCursor(bytes, DEFAULT_CONFIG)
    const result = readToEnvoyConn(bc)
    if (bc.offset < bc.view.byteLength) {
        throw new bare.BareError(bc.offset, "remaining bytes")
    }
    return result
}

/**
 * MARK: To Gateway
 */
export type ToGatewayPong = {
    readonly requestId: RequestId
    readonly ts: i64
}

export function readToGatewayPong(bc: bare.ByteCursor): ToGatewayPong {
    return {
        requestId: readRequestId(bc),
        ts: bare.readI64(bc),
    }
}

export function writeToGatewayPong(bc: bare.ByteCursor, x: ToGatewayPong): void {
    writeRequestId(bc, x.requestId)
    bare.writeI64(bc, x.ts)
}

export type ToGateway =
    | { readonly tag: "ToGatewayPong"; readonly val: ToGatewayPong }
    | { readonly tag: "ToRivetTunnelMessage"; readonly val: ToRivetTunnelMessage }

export function readToGateway(bc: bare.ByteCursor): ToGateway {
    const offset = bc.offset
    const tag = bare.readU8(bc)
    switch (tag) {
        case 0:
            return { tag: "ToGatewayPong", val: readToGatewayPong(bc) }
        case 1:
            return { tag: "ToRivetTunnelMessage", val: readToRivetTunnelMessage(bc) }
        default: {
            bc.offset = offset
            throw new bare.BareError(offset, "invalid tag")
        }
    }
}

export function writeToGateway(bc: bare.ByteCursor, x: ToGateway): void {
    switch (x.tag) {
        case "ToGatewayPong": {
            bare.writeU8(bc, 0)
            writeToGatewayPong(bc, x.val)
            break
        }
        case "ToRivetTunnelMessage": {
            bare.writeU8(bc, 1)
            writeToRivetTunnelMessage(bc, x.val)
            break
        }
    }
}

export function encodeToGateway(x: ToGateway, config?: Partial<bare.Config>): Uint8Array {
    const fullConfig = config != null ? bare.Config(config) : DEFAULT_CONFIG
    const bc = new bare.ByteCursor(
        new Uint8Array(fullConfig.initialBufferLength),
        fullConfig,
    )
    writeToGateway(bc, x)
    return new Uint8Array(bc.view.buffer, bc.view.byteOffset, bc.offset)
}

export function decodeToGateway(bytes: Uint8Array): ToGateway {
    const bc = new bare.ByteCursor(bytes, DEFAULT_CONFIG)
    const result = readToGateway(bc)
    if (bc.offset < bc.view.byteLength) {
        throw new bare.BareError(bc.offset, "remaining bytes")
    }
    return result
}

/**
 * MARK: To Outbound
 */
export type ToOutboundActorStart = {
    readonly namespaceId: Id
    readonly poolName: string
    readonly checkpoint: ActorCheckpoint
    readonly actorConfig: ActorConfig
}

export function readToOutboundActorStart(bc: bare.ByteCursor): ToOutboundActorStart {
    return {
        namespaceId: readId(bc),
        poolName: bare.readString(bc),
        checkpoint: readActorCheckpoint(bc),
        actorConfig: readActorConfig(bc),
    }
}

export function writeToOutboundActorStart(bc: bare.ByteCursor, x: ToOutboundActorStart): void {
    writeId(bc, x.namespaceId)
    bare.writeString(bc, x.poolName)
    writeActorCheckpoint(bc, x.checkpoint)
    writeActorConfig(bc, x.actorConfig)
}

export type ToOutbound =
    | { readonly tag: "ToOutboundActorStart"; readonly val: ToOutboundActorStart }

export function readToOutbound(bc: bare.ByteCursor): ToOutbound {
    const offset = bc.offset
    const tag = bare.readU8(bc)
    switch (tag) {
        case 0:
            return { tag: "ToOutboundActorStart", val: readToOutboundActorStart(bc) }
        default: {
            bc.offset = offset
            throw new bare.BareError(offset, "invalid tag")
        }
    }
}

export function writeToOutbound(bc: bare.ByteCursor, x: ToOutbound): void {
    switch (x.tag) {
        case "ToOutboundActorStart": {
            bare.writeU8(bc, 0)
            writeToOutboundActorStart(bc, x.val)
            break
        }
    }
}

export function encodeToOutbound(x: ToOutbound, config?: Partial<bare.Config>): Uint8Array {
    const fullConfig = config != null ? bare.Config(config) : DEFAULT_CONFIG
    const bc = new bare.ByteCursor(
        new Uint8Array(fullConfig.initialBufferLength),
        fullConfig,
    )
    writeToOutbound(bc, x)
    return new Uint8Array(bc.view.buffer, bc.view.byteOffset, bc.offset)
}

export function decodeToOutbound(bytes: Uint8Array): ToOutbound {
    const bc = new bare.ByteCursor(bytes, DEFAULT_CONFIG)
    const result = readToOutbound(bc)
    if (bc.offset < bc.view.byteLength) {
        throw new bare.BareError(bc.offset, "remaining bytes")
    }
    return result
}


function assert(condition: boolean, message?: string): asserts condition {
    if (!condition) throw new Error(message ?? "Assertion failed")
}

export const VERSION = 2;