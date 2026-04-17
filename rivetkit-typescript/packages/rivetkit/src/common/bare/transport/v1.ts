// Vendored BARE codec. Keep the wire format compatible with the existing runtime.
import * as bare from "@rivetkit/bare-ts"

const config = /* @__PURE__ */ bare.Config({})

export type u32 = number
export type u64 = bigint

export type WorkflowCbor = ArrayBuffer

export function readWorkflowCbor(bc: bare.ByteCursor): WorkflowCbor {
    return bare.readData(bc)
}

export function writeWorkflowCbor(bc: bare.ByteCursor, x: WorkflowCbor): void {
    bare.writeData(bc, x)
}

export type WorkflowNameIndex = u32

export function readWorkflowNameIndex(bc: bare.ByteCursor): WorkflowNameIndex {
    return bare.readU32(bc)
}

export function writeWorkflowNameIndex(bc: bare.ByteCursor, x: WorkflowNameIndex): void {
    bare.writeU32(bc, x)
}

export type WorkflowLoopIterationMarker = {
    readonly loop: WorkflowNameIndex,
    readonly iteration: u32,
}

export function readWorkflowLoopIterationMarker(bc: bare.ByteCursor): WorkflowLoopIterationMarker {
    return {
        loop: readWorkflowNameIndex(bc),
        iteration: bare.readU32(bc),
    }
}

export function writeWorkflowLoopIterationMarker(bc: bare.ByteCursor, x: WorkflowLoopIterationMarker): void {
    writeWorkflowNameIndex(bc, x.loop)
    bare.writeU32(bc, x.iteration)
}

export type WorkflowPathSegment =
    | { readonly tag: "WorkflowNameIndex", readonly val: WorkflowNameIndex }
    | { readonly tag: "WorkflowLoopIterationMarker", readonly val: WorkflowLoopIterationMarker }

export function readWorkflowPathSegment(bc: bare.ByteCursor): WorkflowPathSegment {
    const offset = bc.offset
    const tag = bare.readU8(bc)
    switch (tag) {
        case 0:
            return { tag: "WorkflowNameIndex", val: readWorkflowNameIndex(bc) }
        case 1:
            return { tag: "WorkflowLoopIterationMarker", val: readWorkflowLoopIterationMarker(bc) }
        default: {
            bc.offset = offset
            throw new bare.BareError(offset, "invalid tag")
        }
    }
}

export function writeWorkflowPathSegment(bc: bare.ByteCursor, x: WorkflowPathSegment): void {
    switch (x.tag) {
        case "WorkflowNameIndex": {
            bare.writeU8(bc, 0)
            writeWorkflowNameIndex(bc, x.val)
            break
        }
        case "WorkflowLoopIterationMarker": {
            bare.writeU8(bc, 1)
            writeWorkflowLoopIterationMarker(bc, x.val)
            break
        }
    }
}

export type WorkflowLocation = readonly WorkflowPathSegment[]

export function readWorkflowLocation(bc: bare.ByteCursor): WorkflowLocation {
    const len = bare.readUintSafe(bc)
    if (len === 0) { return [] }
    const result = [readWorkflowPathSegment(bc)]
    for (let i = 1; i < len; i++) {
        result[i] = readWorkflowPathSegment(bc)
    }
    return result
}

export function writeWorkflowLocation(bc: bare.ByteCursor, x: WorkflowLocation): void {
    bare.writeUintSafe(bc, x.length)
    for (let i = 0; i < x.length; i++) {
        writeWorkflowPathSegment(bc, x[i])
    }
}

export enum WorkflowEntryStatus {
    PENDING = "PENDING",
    RUNNING = "RUNNING",
    COMPLETED = "COMPLETED",
    FAILED = "FAILED",
    EXHAUSTED = "EXHAUSTED",
}

export function readWorkflowEntryStatus(bc: bare.ByteCursor): WorkflowEntryStatus {
    const offset = bc.offset
    const tag = bare.readU8(bc)
    switch (tag) {
        case 0:
            return WorkflowEntryStatus.PENDING
        case 1:
            return WorkflowEntryStatus.RUNNING
        case 2:
            return WorkflowEntryStatus.COMPLETED
        case 3:
            return WorkflowEntryStatus.FAILED
        case 4:
            return WorkflowEntryStatus.EXHAUSTED
        default: {
            bc.offset = offset
            throw new bare.BareError(offset, "invalid tag")
        }
    }
}

export function writeWorkflowEntryStatus(bc: bare.ByteCursor, x: WorkflowEntryStatus): void {
    switch (x) {
        case WorkflowEntryStatus.PENDING: {
            bare.writeU8(bc, 0)
            break
        }
        case WorkflowEntryStatus.RUNNING: {
            bare.writeU8(bc, 1)
            break
        }
        case WorkflowEntryStatus.COMPLETED: {
            bare.writeU8(bc, 2)
            break
        }
        case WorkflowEntryStatus.FAILED: {
            bare.writeU8(bc, 3)
            break
        }
        case WorkflowEntryStatus.EXHAUSTED: {
            bare.writeU8(bc, 4)
            break
        }
    }
}

export enum WorkflowSleepState {
    PENDING = "PENDING",
    COMPLETED = "COMPLETED",
    INTERRUPTED = "INTERRUPTED",
}

export function readWorkflowSleepState(bc: bare.ByteCursor): WorkflowSleepState {
    const offset = bc.offset
    const tag = bare.readU8(bc)
    switch (tag) {
        case 0:
            return WorkflowSleepState.PENDING
        case 1:
            return WorkflowSleepState.COMPLETED
        case 2:
            return WorkflowSleepState.INTERRUPTED
        default: {
            bc.offset = offset
            throw new bare.BareError(offset, "invalid tag")
        }
    }
}

export function writeWorkflowSleepState(bc: bare.ByteCursor, x: WorkflowSleepState): void {
    switch (x) {
        case WorkflowSleepState.PENDING: {
            bare.writeU8(bc, 0)
            break
        }
        case WorkflowSleepState.COMPLETED: {
            bare.writeU8(bc, 1)
            break
        }
        case WorkflowSleepState.INTERRUPTED: {
            bare.writeU8(bc, 2)
            break
        }
    }
}

export enum WorkflowBranchStatusType {
    PENDING = "PENDING",
    RUNNING = "RUNNING",
    COMPLETED = "COMPLETED",
    FAILED = "FAILED",
    CANCELLED = "CANCELLED",
}

export function readWorkflowBranchStatusType(bc: bare.ByteCursor): WorkflowBranchStatusType {
    const offset = bc.offset
    const tag = bare.readU8(bc)
    switch (tag) {
        case 0:
            return WorkflowBranchStatusType.PENDING
        case 1:
            return WorkflowBranchStatusType.RUNNING
        case 2:
            return WorkflowBranchStatusType.COMPLETED
        case 3:
            return WorkflowBranchStatusType.FAILED
        case 4:
            return WorkflowBranchStatusType.CANCELLED
        default: {
            bc.offset = offset
            throw new bare.BareError(offset, "invalid tag")
        }
    }
}

export function writeWorkflowBranchStatusType(bc: bare.ByteCursor, x: WorkflowBranchStatusType): void {
    switch (x) {
        case WorkflowBranchStatusType.PENDING: {
            bare.writeU8(bc, 0)
            break
        }
        case WorkflowBranchStatusType.RUNNING: {
            bare.writeU8(bc, 1)
            break
        }
        case WorkflowBranchStatusType.COMPLETED: {
            bare.writeU8(bc, 2)
            break
        }
        case WorkflowBranchStatusType.FAILED: {
            bare.writeU8(bc, 3)
            break
        }
        case WorkflowBranchStatusType.CANCELLED: {
            bare.writeU8(bc, 4)
            break
        }
    }
}

function read0(bc: bare.ByteCursor): WorkflowCbor | null {
    return bare.readBool(bc)
        ? readWorkflowCbor(bc)
        : null
}

function write0(bc: bare.ByteCursor, x: WorkflowCbor | null): void {
    bare.writeBool(bc, x !== null)
    if (x !== null) {
        writeWorkflowCbor(bc, x)
    }
}

function read1(bc: bare.ByteCursor): string | null {
    return bare.readBool(bc)
        ? bare.readString(bc)
        : null
}

function write1(bc: bare.ByteCursor, x: string | null): void {
    bare.writeBool(bc, x !== null)
    if (x !== null) {
        bare.writeString(bc, x)
    }
}

export type WorkflowStepEntry = {
    readonly output: WorkflowCbor | null,
    readonly error: string | null,
}

export function readWorkflowStepEntry(bc: bare.ByteCursor): WorkflowStepEntry {
    return {
        output: read0(bc),
        error: read1(bc),
    }
}

export function writeWorkflowStepEntry(bc: bare.ByteCursor, x: WorkflowStepEntry): void {
    write0(bc, x.output)
    write1(bc, x.error)
}

export type WorkflowLoopEntry = {
    readonly state: WorkflowCbor,
    readonly iteration: u32,
    readonly output: WorkflowCbor | null,
}

export function readWorkflowLoopEntry(bc: bare.ByteCursor): WorkflowLoopEntry {
    return {
        state: readWorkflowCbor(bc),
        iteration: bare.readU32(bc),
        output: read0(bc),
    }
}

export function writeWorkflowLoopEntry(bc: bare.ByteCursor, x: WorkflowLoopEntry): void {
    writeWorkflowCbor(bc, x.state)
    bare.writeU32(bc, x.iteration)
    write0(bc, x.output)
}

export type WorkflowSleepEntry = {
    readonly deadline: u64,
    readonly state: WorkflowSleepState,
}

export function readWorkflowSleepEntry(bc: bare.ByteCursor): WorkflowSleepEntry {
    return {
        deadline: bare.readU64(bc),
        state: readWorkflowSleepState(bc),
    }
}

export function writeWorkflowSleepEntry(bc: bare.ByteCursor, x: WorkflowSleepEntry): void {
    bare.writeU64(bc, x.deadline)
    writeWorkflowSleepState(bc, x.state)
}

export type WorkflowMessageEntry = {
    readonly name: string,
    readonly messageData: WorkflowCbor,
}

export function readWorkflowMessageEntry(bc: bare.ByteCursor): WorkflowMessageEntry {
    return {
        name: bare.readString(bc),
        messageData: readWorkflowCbor(bc),
    }
}

export function writeWorkflowMessageEntry(bc: bare.ByteCursor, x: WorkflowMessageEntry): void {
    bare.writeString(bc, x.name)
    writeWorkflowCbor(bc, x.messageData)
}

export type WorkflowRollbackCheckpointEntry = {
    readonly name: string,
}

export function readWorkflowRollbackCheckpointEntry(bc: bare.ByteCursor): WorkflowRollbackCheckpointEntry {
    return {
        name: bare.readString(bc),
    }
}

export function writeWorkflowRollbackCheckpointEntry(bc: bare.ByteCursor, x: WorkflowRollbackCheckpointEntry): void {
    bare.writeString(bc, x.name)
}

export type WorkflowBranchStatus = {
    readonly status: WorkflowBranchStatusType,
    readonly output: WorkflowCbor | null,
    readonly error: string | null,
}

export function readWorkflowBranchStatus(bc: bare.ByteCursor): WorkflowBranchStatus {
    return {
        status: readWorkflowBranchStatusType(bc),
        output: read0(bc),
        error: read1(bc),
    }
}

export function writeWorkflowBranchStatus(bc: bare.ByteCursor, x: WorkflowBranchStatus): void {
    writeWorkflowBranchStatusType(bc, x.status)
    write0(bc, x.output)
    write1(bc, x.error)
}

function read2(bc: bare.ByteCursor): ReadonlyMap<string, WorkflowBranchStatus> {
    const len = bare.readUintSafe(bc)
    const result = new Map<string, WorkflowBranchStatus>()
    for (let i = 0; i < len; i++) {
        const offset = bc.offset
        const key = bare.readString(bc)
        if (result.has(key)) {
            bc.offset = offset
            throw new bare.BareError(offset, "duplicated key")
        }
        result.set(key, readWorkflowBranchStatus(bc))
    }
    return result
}

function write2(bc: bare.ByteCursor, x: ReadonlyMap<string, WorkflowBranchStatus>): void {
    bare.writeUintSafe(bc, x.size)
    for(const kv of x) {
        bare.writeString(bc, kv[0])
        writeWorkflowBranchStatus(bc, kv[1])
    }
}

export type WorkflowJoinEntry = {
    readonly branches: ReadonlyMap<string, WorkflowBranchStatus>,
}

export function readWorkflowJoinEntry(bc: bare.ByteCursor): WorkflowJoinEntry {
    return {
        branches: read2(bc),
    }
}

export function writeWorkflowJoinEntry(bc: bare.ByteCursor, x: WorkflowJoinEntry): void {
    write2(bc, x.branches)
}

export type WorkflowRaceEntry = {
    readonly winner: string | null,
    readonly branches: ReadonlyMap<string, WorkflowBranchStatus>,
}

export function readWorkflowRaceEntry(bc: bare.ByteCursor): WorkflowRaceEntry {
    return {
        winner: read1(bc),
        branches: read2(bc),
    }
}

export function writeWorkflowRaceEntry(bc: bare.ByteCursor, x: WorkflowRaceEntry): void {
    write1(bc, x.winner)
    write2(bc, x.branches)
}

export type WorkflowRemovedEntry = {
    readonly originalType: string,
    readonly originalName: string | null,
}

export function readWorkflowRemovedEntry(bc: bare.ByteCursor): WorkflowRemovedEntry {
    return {
        originalType: bare.readString(bc),
        originalName: read1(bc),
    }
}

export function writeWorkflowRemovedEntry(bc: bare.ByteCursor, x: WorkflowRemovedEntry): void {
    bare.writeString(bc, x.originalType)
    write1(bc, x.originalName)
}

export type WorkflowEntryKind =
    | { readonly tag: "WorkflowStepEntry", readonly val: WorkflowStepEntry }
    | { readonly tag: "WorkflowLoopEntry", readonly val: WorkflowLoopEntry }
    | { readonly tag: "WorkflowSleepEntry", readonly val: WorkflowSleepEntry }
    | { readonly tag: "WorkflowMessageEntry", readonly val: WorkflowMessageEntry }
    | { readonly tag: "WorkflowRollbackCheckpointEntry", readonly val: WorkflowRollbackCheckpointEntry }
    | { readonly tag: "WorkflowJoinEntry", readonly val: WorkflowJoinEntry }
    | { readonly tag: "WorkflowRaceEntry", readonly val: WorkflowRaceEntry }
    | { readonly tag: "WorkflowRemovedEntry", readonly val: WorkflowRemovedEntry }

export function readWorkflowEntryKind(bc: bare.ByteCursor): WorkflowEntryKind {
    const offset = bc.offset
    const tag = bare.readU8(bc)
    switch (tag) {
        case 0:
            return { tag: "WorkflowStepEntry", val: readWorkflowStepEntry(bc) }
        case 1:
            return { tag: "WorkflowLoopEntry", val: readWorkflowLoopEntry(bc) }
        case 2:
            return { tag: "WorkflowSleepEntry", val: readWorkflowSleepEntry(bc) }
        case 3:
            return { tag: "WorkflowMessageEntry", val: readWorkflowMessageEntry(bc) }
        case 4:
            return { tag: "WorkflowRollbackCheckpointEntry", val: readWorkflowRollbackCheckpointEntry(bc) }
        case 5:
            return { tag: "WorkflowJoinEntry", val: readWorkflowJoinEntry(bc) }
        case 6:
            return { tag: "WorkflowRaceEntry", val: readWorkflowRaceEntry(bc) }
        case 7:
            return { tag: "WorkflowRemovedEntry", val: readWorkflowRemovedEntry(bc) }
        default: {
            bc.offset = offset
            throw new bare.BareError(offset, "invalid tag")
        }
    }
}

export function writeWorkflowEntryKind(bc: bare.ByteCursor, x: WorkflowEntryKind): void {
    switch (x.tag) {
        case "WorkflowStepEntry": {
            bare.writeU8(bc, 0)
            writeWorkflowStepEntry(bc, x.val)
            break
        }
        case "WorkflowLoopEntry": {
            bare.writeU8(bc, 1)
            writeWorkflowLoopEntry(bc, x.val)
            break
        }
        case "WorkflowSleepEntry": {
            bare.writeU8(bc, 2)
            writeWorkflowSleepEntry(bc, x.val)
            break
        }
        case "WorkflowMessageEntry": {
            bare.writeU8(bc, 3)
            writeWorkflowMessageEntry(bc, x.val)
            break
        }
        case "WorkflowRollbackCheckpointEntry": {
            bare.writeU8(bc, 4)
            writeWorkflowRollbackCheckpointEntry(bc, x.val)
            break
        }
        case "WorkflowJoinEntry": {
            bare.writeU8(bc, 5)
            writeWorkflowJoinEntry(bc, x.val)
            break
        }
        case "WorkflowRaceEntry": {
            bare.writeU8(bc, 6)
            writeWorkflowRaceEntry(bc, x.val)
            break
        }
        case "WorkflowRemovedEntry": {
            bare.writeU8(bc, 7)
            writeWorkflowRemovedEntry(bc, x.val)
            break
        }
    }
}

export type WorkflowEntry = {
    readonly id: string,
    readonly location: WorkflowLocation,
    readonly kind: WorkflowEntryKind,
}

export function readWorkflowEntry(bc: bare.ByteCursor): WorkflowEntry {
    return {
        id: bare.readString(bc),
        location: readWorkflowLocation(bc),
        kind: readWorkflowEntryKind(bc),
    }
}

export function writeWorkflowEntry(bc: bare.ByteCursor, x: WorkflowEntry): void {
    bare.writeString(bc, x.id)
    writeWorkflowLocation(bc, x.location)
    writeWorkflowEntryKind(bc, x.kind)
}

function read3(bc: bare.ByteCursor): u64 | null {
    return bare.readBool(bc)
        ? bare.readU64(bc)
        : null
}

function write3(bc: bare.ByteCursor, x: u64 | null): void {
    bare.writeBool(bc, x !== null)
    if (x !== null) {
        bare.writeU64(bc, x)
    }
}

export type WorkflowEntryMetadata = {
    readonly status: WorkflowEntryStatus,
    readonly error: string | null,
    readonly attempts: u32,
    readonly lastAttemptAt: u64,
    readonly createdAt: u64,
    readonly completedAt: u64 | null,
    readonly rollbackCompletedAt: u64 | null,
    readonly rollbackError: string | null,
}

export function readWorkflowEntryMetadata(bc: bare.ByteCursor): WorkflowEntryMetadata {
    return {
        status: readWorkflowEntryStatus(bc),
        error: read1(bc),
        attempts: bare.readU32(bc),
        lastAttemptAt: bare.readU64(bc),
        createdAt: bare.readU64(bc),
        completedAt: read3(bc),
        rollbackCompletedAt: read3(bc),
        rollbackError: read1(bc),
    }
}

export function writeWorkflowEntryMetadata(bc: bare.ByteCursor, x: WorkflowEntryMetadata): void {
    writeWorkflowEntryStatus(bc, x.status)
    write1(bc, x.error)
    bare.writeU32(bc, x.attempts)
    bare.writeU64(bc, x.lastAttemptAt)
    bare.writeU64(bc, x.createdAt)
    write3(bc, x.completedAt)
    write3(bc, x.rollbackCompletedAt)
    write1(bc, x.rollbackError)
}

function read4(bc: bare.ByteCursor): readonly string[] {
    const len = bare.readUintSafe(bc)
    if (len === 0) { return [] }
    const result = [bare.readString(bc)]
    for (let i = 1; i < len; i++) {
        result[i] = bare.readString(bc)
    }
    return result
}

function write4(bc: bare.ByteCursor, x: readonly string[]): void {
    bare.writeUintSafe(bc, x.length)
    for (let i = 0; i < x.length; i++) {
        bare.writeString(bc, x[i])
    }
}

function read5(bc: bare.ByteCursor): readonly WorkflowEntry[] {
    const len = bare.readUintSafe(bc)
    if (len === 0) { return [] }
    const result = [readWorkflowEntry(bc)]
    for (let i = 1; i < len; i++) {
        result[i] = readWorkflowEntry(bc)
    }
    return result
}

function write5(bc: bare.ByteCursor, x: readonly WorkflowEntry[]): void {
    bare.writeUintSafe(bc, x.length)
    for (let i = 0; i < x.length; i++) {
        writeWorkflowEntry(bc, x[i])
    }
}

function read6(bc: bare.ByteCursor): ReadonlyMap<string, WorkflowEntryMetadata> {
    const len = bare.readUintSafe(bc)
    const result = new Map<string, WorkflowEntryMetadata>()
    for (let i = 0; i < len; i++) {
        const offset = bc.offset
        const key = bare.readString(bc)
        if (result.has(key)) {
            bc.offset = offset
            throw new bare.BareError(offset, "duplicated key")
        }
        result.set(key, readWorkflowEntryMetadata(bc))
    }
    return result
}

function write6(bc: bare.ByteCursor, x: ReadonlyMap<string, WorkflowEntryMetadata>): void {
    bare.writeUintSafe(bc, x.size)
    for(const kv of x) {
        bare.writeString(bc, kv[0])
        writeWorkflowEntryMetadata(bc, kv[1])
    }
}

export type WorkflowHistory = {
    readonly nameRegistry: readonly string[],
    readonly entries: readonly WorkflowEntry[],
    readonly entryMetadata: ReadonlyMap<string, WorkflowEntryMetadata>,
}

export function readWorkflowHistory(bc: bare.ByteCursor): WorkflowHistory {
    return {
        nameRegistry: read4(bc),
        entries: read5(bc),
        entryMetadata: read6(bc),
    }
}

export function writeWorkflowHistory(bc: bare.ByteCursor, x: WorkflowHistory): void {
    write4(bc, x.nameRegistry)
    write5(bc, x.entries)
    write6(bc, x.entryMetadata)
}

export function encodeWorkflowHistory(x: WorkflowHistory): Uint8Array {
    const bc = new bare.ByteCursor(
        new Uint8Array(config.initialBufferLength),
        config
    )
    writeWorkflowHistory(bc, x)
    return new Uint8Array(bc.view.buffer, bc.view.byteOffset, bc.offset)
}

export function decodeWorkflowHistory(bytes: Uint8Array): WorkflowHistory {
    const bc = new bare.ByteCursor(bytes, config)
    const result = readWorkflowHistory(bc)
    if (bc.offset < bc.view.byteLength) {
        throw new bare.BareError(bc.offset, "remaining bytes")
    }
    return result
}


function assert(condition: boolean, message?: string): asserts condition {
    if (!condition) throw new Error(message ?? "Assertion failed")
}
