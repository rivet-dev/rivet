import * as bare from "@bare-ts/lib"

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
    readonly createTs: i64
}

export function readKvMetadata(bc: bare.ByteCursor): KvMetadata {
    return {
        version: bare.readData(bc),
        createTs: bare.readI64(bc),
    }
}

export function writeKvMetadata(bc: bare.ByteCursor, x: KvMetadata): void {
    bare.writeData(bc, x.version)
    bare.writeI64(bc, x.createTs)
}

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

export type RunnerAddressHttp = {
    readonly hostname: string
    readonly port: u16
}

export function readRunnerAddressHttp(bc: bare.ByteCursor): RunnerAddressHttp {
    return {
        hostname: bare.readString(bc),
        port: bare.readU16(bc),
    }
}

export function writeRunnerAddressHttp(bc: bare.ByteCursor, x: RunnerAddressHttp): void {
    bare.writeString(bc, x.hostname)
    bare.writeU16(bc, x.port)
}

export type RunnerAddressTcp = {
    readonly hostname: string
    readonly port: u16
}

export function readRunnerAddressTcp(bc: bare.ByteCursor): RunnerAddressTcp {
    return {
        hostname: bare.readString(bc),
        port: bare.readU16(bc),
    }
}

export function writeRunnerAddressTcp(bc: bare.ByteCursor, x: RunnerAddressTcp): void {
    bare.writeString(bc, x.hostname)
    bare.writeU16(bc, x.port)
}

export type RunnerAddressUdp = {
    readonly hostname: string
    readonly port: u16
}

export function readRunnerAddressUdp(bc: bare.ByteCursor): RunnerAddressUdp {
    return {
        hostname: bare.readString(bc),
        port: bare.readU16(bc),
    }
}

export function writeRunnerAddressUdp(bc: bare.ByteCursor, x: RunnerAddressUdp): void {
    bare.writeString(bc, x.hostname)
    bare.writeU16(bc, x.port)
}

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

export type ActorStateRunning = null

function read0(bc: bare.ByteCursor): string | null {
    return bare.readBool(bc) ? bare.readString(bc) : null
}

function write0(bc: bare.ByteCursor, x: string | null): void {
    bare.writeBool(bc, x != null)
    if (x != null) {
        bare.writeString(bc, x)
    }
}

export type ActorStateStopped = {
    readonly code: StopCode
    readonly message: string | null
}

export function readActorStateStopped(bc: bare.ByteCursor): ActorStateStopped {
    return {
        code: readStopCode(bc),
        message: read0(bc),
    }
}

export function writeActorStateStopped(bc: bare.ByteCursor, x: ActorStateStopped): void {
    writeStopCode(bc, x.code)
    write0(bc, x.message)
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

export type EventActorIntent = {
    readonly actorId: Id
    readonly generation: u32
    readonly intent: ActorIntent
}

export function readEventActorIntent(bc: bare.ByteCursor): EventActorIntent {
    return {
        actorId: readId(bc),
        generation: bare.readU32(bc),
        intent: readActorIntent(bc),
    }
}

export function writeEventActorIntent(bc: bare.ByteCursor, x: EventActorIntent): void {
    writeId(bc, x.actorId)
    bare.writeU32(bc, x.generation)
    writeActorIntent(bc, x.intent)
}

export type EventActorStateUpdate = {
    readonly actorId: Id
    readonly generation: u32
    readonly state: ActorState
}

export function readEventActorStateUpdate(bc: bare.ByteCursor): EventActorStateUpdate {
    return {
        actorId: readId(bc),
        generation: bare.readU32(bc),
        state: readActorState(bc),
    }
}

export function writeEventActorStateUpdate(bc: bare.ByteCursor, x: EventActorStateUpdate): void {
    writeId(bc, x.actorId)
    bare.writeU32(bc, x.generation)
    writeActorState(bc, x.state)
}

function read1(bc: bare.ByteCursor): i64 | null {
    return bare.readBool(bc) ? bare.readI64(bc) : null
}

function write1(bc: bare.ByteCursor, x: i64 | null): void {
    bare.writeBool(bc, x != null)
    if (x != null) {
        bare.writeI64(bc, x)
    }
}

export type EventActorSetAlarm = {
    readonly actorId: Id
    readonly generation: u32
    readonly alarmTs: i64 | null
}

export function readEventActorSetAlarm(bc: bare.ByteCursor): EventActorSetAlarm {
    return {
        actorId: readId(bc),
        generation: bare.readU32(bc),
        alarmTs: read1(bc),
    }
}

export function writeEventActorSetAlarm(bc: bare.ByteCursor, x: EventActorSetAlarm): void {
    writeId(bc, x.actorId)
    bare.writeU32(bc, x.generation)
    write1(bc, x.alarmTs)
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
    readonly index: i64
    readonly inner: Event
}

export function readEventWrapper(bc: bare.ByteCursor): EventWrapper {
    return {
        index: bare.readI64(bc),
        inner: readEvent(bc),
    }
}

export function writeEventWrapper(bc: bare.ByteCursor, x: EventWrapper): void {
    bare.writeI64(bc, x.index)
    writeEvent(bc, x.inner)
}

function read2(bc: bare.ByteCursor): ArrayBuffer | null {
    return bare.readBool(bc) ? bare.readData(bc) : null
}

function write2(bc: bare.ByteCursor, x: ArrayBuffer | null): void {
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
        key: read0(bc),
        createTs: bare.readI64(bc),
        input: read2(bc),
    }
}

export function writeActorConfig(bc: bare.ByteCursor, x: ActorConfig): void {
    bare.writeString(bc, x.name)
    write0(bc, x.key)
    bare.writeI64(bc, x.createTs)
    write2(bc, x.input)
}

export type CommandStartActor = {
    readonly actorId: Id
    readonly generation: u32
    readonly config: ActorConfig
}

export function readCommandStartActor(bc: bare.ByteCursor): CommandStartActor {
    return {
        actorId: readId(bc),
        generation: bare.readU32(bc),
        config: readActorConfig(bc),
    }
}

export function writeCommandStartActor(bc: bare.ByteCursor, x: CommandStartActor): void {
    writeId(bc, x.actorId)
    bare.writeU32(bc, x.generation)
    writeActorConfig(bc, x.config)
}

export type CommandStopActor = {
    readonly actorId: Id
    readonly generation: u32
}

export function readCommandStopActor(bc: bare.ByteCursor): CommandStopActor {
    return {
        actorId: readId(bc),
        generation: bare.readU32(bc),
    }
}

export function writeCommandStopActor(bc: bare.ByteCursor, x: CommandStopActor): void {
    writeId(bc, x.actorId)
    bare.writeU32(bc, x.generation)
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
    readonly index: i64
    readonly inner: Command
}

export function readCommandWrapper(bc: bare.ByteCursor): CommandWrapper {
    return {
        index: bare.readI64(bc),
        inner: readCommand(bc),
    }
}

export function writeCommandWrapper(bc: bare.ByteCursor, x: CommandWrapper): void {
    bare.writeI64(bc, x.index)
    writeCommand(bc, x.inner)
}

function read3(bc: bare.ByteCursor): Id | null {
    return bare.readBool(bc) ? readId(bc) : null
}

function write3(bc: bare.ByteCursor, x: Id | null): void {
    bare.writeBool(bc, x != null)
    if (x != null) {
        writeId(bc, x)
    }
}

function read4(bc: bare.ByteCursor): ReadonlyMap<string, RunnerAddressHttp> {
    const len = bare.readUintSafe(bc)
    const result = new Map<string, RunnerAddressHttp>()
    for (let i = 0; i < len; i++) {
        const offset = bc.offset
        const key = bare.readString(bc)
        if (result.has(key)) {
            bc.offset = offset
            throw new bare.BareError(offset, "duplicated key")
        }
        result.set(key, readRunnerAddressHttp(bc))
    }
    return result
}

function write4(bc: bare.ByteCursor, x: ReadonlyMap<string, RunnerAddressHttp>): void {
    bare.writeUintSafe(bc, x.size)
    for (const kv of x) {
        bare.writeString(bc, kv[0])
        writeRunnerAddressHttp(bc, kv[1])
    }
}

function read5(bc: bare.ByteCursor): ReadonlyMap<string, RunnerAddressHttp> | null {
    return bare.readBool(bc) ? read4(bc) : null
}

function write5(bc: bare.ByteCursor, x: ReadonlyMap<string, RunnerAddressHttp> | null): void {
    bare.writeBool(bc, x != null)
    if (x != null) {
        write4(bc, x)
    }
}

function read6(bc: bare.ByteCursor): ReadonlyMap<string, RunnerAddressTcp> {
    const len = bare.readUintSafe(bc)
    const result = new Map<string, RunnerAddressTcp>()
    for (let i = 0; i < len; i++) {
        const offset = bc.offset
        const key = bare.readString(bc)
        if (result.has(key)) {
            bc.offset = offset
            throw new bare.BareError(offset, "duplicated key")
        }
        result.set(key, readRunnerAddressTcp(bc))
    }
    return result
}

function write6(bc: bare.ByteCursor, x: ReadonlyMap<string, RunnerAddressTcp>): void {
    bare.writeUintSafe(bc, x.size)
    for (const kv of x) {
        bare.writeString(bc, kv[0])
        writeRunnerAddressTcp(bc, kv[1])
    }
}

function read7(bc: bare.ByteCursor): ReadonlyMap<string, RunnerAddressTcp> | null {
    return bare.readBool(bc) ? read6(bc) : null
}

function write7(bc: bare.ByteCursor, x: ReadonlyMap<string, RunnerAddressTcp> | null): void {
    bare.writeBool(bc, x != null)
    if (x != null) {
        write6(bc, x)
    }
}

function read8(bc: bare.ByteCursor): ReadonlyMap<string, RunnerAddressUdp> {
    const len = bare.readUintSafe(bc)
    const result = new Map<string, RunnerAddressUdp>()
    for (let i = 0; i < len; i++) {
        const offset = bc.offset
        const key = bare.readString(bc)
        if (result.has(key)) {
            bc.offset = offset
            throw new bare.BareError(offset, "duplicated key")
        }
        result.set(key, readRunnerAddressUdp(bc))
    }
    return result
}

function write8(bc: bare.ByteCursor, x: ReadonlyMap<string, RunnerAddressUdp>): void {
    bare.writeUintSafe(bc, x.size)
    for (const kv of x) {
        bare.writeString(bc, kv[0])
        writeRunnerAddressUdp(bc, kv[1])
    }
}

function read9(bc: bare.ByteCursor): ReadonlyMap<string, RunnerAddressUdp> | null {
    return bare.readBool(bc) ? read8(bc) : null
}

function write9(bc: bare.ByteCursor, x: ReadonlyMap<string, RunnerAddressUdp> | null): void {
    bare.writeBool(bc, x != null)
    if (x != null) {
        write8(bc, x)
    }
}

function read10(bc: bare.ByteCursor): ReadonlyMap<string, ActorName> {
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

function write10(bc: bare.ByteCursor, x: ReadonlyMap<string, ActorName>): void {
    bare.writeUintSafe(bc, x.size)
    for (const kv of x) {
        bare.writeString(bc, kv[0])
        writeActorName(bc, kv[1])
    }
}

function read11(bc: bare.ByteCursor): ReadonlyMap<string, ActorName> | null {
    return bare.readBool(bc) ? read10(bc) : null
}

function write11(bc: bare.ByteCursor, x: ReadonlyMap<string, ActorName> | null): void {
    bare.writeBool(bc, x != null)
    if (x != null) {
        write10(bc, x)
    }
}

function read12(bc: bare.ByteCursor): Json | null {
    return bare.readBool(bc) ? readJson(bc) : null
}

function write12(bc: bare.ByteCursor, x: Json | null): void {
    bare.writeBool(bc, x != null)
    if (x != null) {
        writeJson(bc, x)
    }
}

export type ToServerInit = {
    readonly runnerId: Id | null
    readonly name: string
    readonly key: string
    readonly version: u32
    readonly totalSlots: u32
    readonly addressesHttp: ReadonlyMap<string, RunnerAddressHttp> | null
    readonly addressesTcp: ReadonlyMap<string, RunnerAddressTcp> | null
    readonly addressesUdp: ReadonlyMap<string, RunnerAddressUdp> | null
    readonly lastCommandIdx: i64 | null
    readonly prepopulateActorNames: ReadonlyMap<string, ActorName> | null
    readonly metadata: Json | null
}

export function readToServerInit(bc: bare.ByteCursor): ToServerInit {
    return {
        runnerId: read3(bc),
        name: bare.readString(bc),
        key: bare.readString(bc),
        version: bare.readU32(bc),
        totalSlots: bare.readU32(bc),
        addressesHttp: read5(bc),
        addressesTcp: read7(bc),
        addressesUdp: read9(bc),
        lastCommandIdx: read1(bc),
        prepopulateActorNames: read11(bc),
        metadata: read12(bc),
    }
}

export function writeToServerInit(bc: bare.ByteCursor, x: ToServerInit): void {
    write3(bc, x.runnerId)
    bare.writeString(bc, x.name)
    bare.writeString(bc, x.key)
    bare.writeU32(bc, x.version)
    bare.writeU32(bc, x.totalSlots)
    write5(bc, x.addressesHttp)
    write7(bc, x.addressesTcp)
    write9(bc, x.addressesUdp)
    write1(bc, x.lastCommandIdx)
    write11(bc, x.prepopulateActorNames)
    write12(bc, x.metadata)
}

export type ToServerEvents = readonly EventWrapper[]

export function readToServerEvents(bc: bare.ByteCursor): ToServerEvents {
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

export function writeToServerEvents(bc: bare.ByteCursor, x: ToServerEvents): void {
    bare.writeUintSafe(bc, x.length)
    for (let i = 0; i < x.length; i++) {
        writeEventWrapper(bc, x[i])
    }
}

export type ToServerAckCommands = {
    readonly lastCommandIdx: i64
}

export function readToServerAckCommands(bc: bare.ByteCursor): ToServerAckCommands {
    return {
        lastCommandIdx: bare.readI64(bc),
    }
}

export function writeToServerAckCommands(bc: bare.ByteCursor, x: ToServerAckCommands): void {
    bare.writeI64(bc, x.lastCommandIdx)
}

export type ToServerStopping = null

export type ToServerPing = {
    readonly ts: i64
}

export function readToServerPing(bc: bare.ByteCursor): ToServerPing {
    return {
        ts: bare.readI64(bc),
    }
}

export function writeToServerPing(bc: bare.ByteCursor, x: ToServerPing): void {
    bare.writeI64(bc, x.ts)
}

function read13(bc: bare.ByteCursor): readonly KvKey[] {
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

function write13(bc: bare.ByteCursor, x: readonly KvKey[]): void {
    bare.writeUintSafe(bc, x.length)
    for (let i = 0; i < x.length; i++) {
        writeKvKey(bc, x[i])
    }
}

export type KvGetRequest = {
    readonly keys: readonly KvKey[]
}

export function readKvGetRequest(bc: bare.ByteCursor): KvGetRequest {
    return {
        keys: read13(bc),
    }
}

export function writeKvGetRequest(bc: bare.ByteCursor, x: KvGetRequest): void {
    write13(bc, x.keys)
}

function read14(bc: bare.ByteCursor): boolean | null {
    return bare.readBool(bc) ? bare.readBool(bc) : null
}

function write14(bc: bare.ByteCursor, x: boolean | null): void {
    bare.writeBool(bc, x != null)
    if (x != null) {
        bare.writeBool(bc, x)
    }
}

function read15(bc: bare.ByteCursor): u64 | null {
    return bare.readBool(bc) ? bare.readU64(bc) : null
}

function write15(bc: bare.ByteCursor, x: u64 | null): void {
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
        reverse: read14(bc),
        limit: read15(bc),
    }
}

export function writeKvListRequest(bc: bare.ByteCursor, x: KvListRequest): void {
    writeKvListQuery(bc, x.query)
    write14(bc, x.reverse)
    write15(bc, x.limit)
}

function read16(bc: bare.ByteCursor): readonly KvValue[] {
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

function write16(bc: bare.ByteCursor, x: readonly KvValue[]): void {
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
        keys: read13(bc),
        values: read16(bc),
    }
}

export function writeKvPutRequest(bc: bare.ByteCursor, x: KvPutRequest): void {
    write13(bc, x.keys)
    write16(bc, x.values)
}

export type KvDeleteRequest = {
    readonly keys: readonly KvKey[]
}

export function readKvDeleteRequest(bc: bare.ByteCursor): KvDeleteRequest {
    return {
        keys: read13(bc),
    }
}

export function writeKvDeleteRequest(bc: bare.ByteCursor, x: KvDeleteRequest): void {
    write13(bc, x.keys)
}

export type KvDropRequest = null

export type KvRequestData =
    | { readonly tag: "KvGetRequest"; readonly val: KvGetRequest }
    | { readonly tag: "KvListRequest"; readonly val: KvListRequest }
    | { readonly tag: "KvPutRequest"; readonly val: KvPutRequest }
    | { readonly tag: "KvDeleteRequest"; readonly val: KvDeleteRequest }
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
        case "KvDropRequest": {
            bare.writeU8(bc, 4)
            break
        }
    }
}

export type ToServerKvRequest = {
    readonly actorId: Id
    readonly requestId: u32
    readonly data: KvRequestData
}

export function readToServerKvRequest(bc: bare.ByteCursor): ToServerKvRequest {
    return {
        actorId: readId(bc),
        requestId: bare.readU32(bc),
        data: readKvRequestData(bc),
    }
}

export function writeToServerKvRequest(bc: bare.ByteCursor, x: ToServerKvRequest): void {
    writeId(bc, x.actorId)
    bare.writeU32(bc, x.requestId)
    writeKvRequestData(bc, x.data)
}

export type ToServer =
    | { readonly tag: "ToServerInit"; readonly val: ToServerInit }
    | { readonly tag: "ToServerEvents"; readonly val: ToServerEvents }
    | { readonly tag: "ToServerAckCommands"; readonly val: ToServerAckCommands }
    | { readonly tag: "ToServerStopping"; readonly val: ToServerStopping }
    | { readonly tag: "ToServerPing"; readonly val: ToServerPing }
    | { readonly tag: "ToServerKvRequest"; readonly val: ToServerKvRequest }

export function readToServer(bc: bare.ByteCursor): ToServer {
    const offset = bc.offset
    const tag = bare.readU8(bc)
    switch (tag) {
        case 0:
            return { tag: "ToServerInit", val: readToServerInit(bc) }
        case 1:
            return { tag: "ToServerEvents", val: readToServerEvents(bc) }
        case 2:
            return { tag: "ToServerAckCommands", val: readToServerAckCommands(bc) }
        case 3:
            return { tag: "ToServerStopping", val: null }
        case 4:
            return { tag: "ToServerPing", val: readToServerPing(bc) }
        case 5:
            return { tag: "ToServerKvRequest", val: readToServerKvRequest(bc) }
        default: {
            bc.offset = offset
            throw new bare.BareError(offset, "invalid tag")
        }
    }
}

export function writeToServer(bc: bare.ByteCursor, x: ToServer): void {
    switch (x.tag) {
        case "ToServerInit": {
            bare.writeU8(bc, 0)
            writeToServerInit(bc, x.val)
            break
        }
        case "ToServerEvents": {
            bare.writeU8(bc, 1)
            writeToServerEvents(bc, x.val)
            break
        }
        case "ToServerAckCommands": {
            bare.writeU8(bc, 2)
            writeToServerAckCommands(bc, x.val)
            break
        }
        case "ToServerStopping": {
            bare.writeU8(bc, 3)
            break
        }
        case "ToServerPing": {
            bare.writeU8(bc, 4)
            writeToServerPing(bc, x.val)
            break
        }
        case "ToServerKvRequest": {
            bare.writeU8(bc, 5)
            writeToServerKvRequest(bc, x.val)
            break
        }
    }
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

export type ProtocolMetadata = {
    readonly runnerLostThreshold: i64
}

export function readProtocolMetadata(bc: bare.ByteCursor): ProtocolMetadata {
    return {
        runnerLostThreshold: bare.readI64(bc),
    }
}

export function writeProtocolMetadata(bc: bare.ByteCursor, x: ProtocolMetadata): void {
    bare.writeI64(bc, x.runnerLostThreshold)
}

export type ToClientInit = {
    readonly runnerId: Id
    readonly lastEventIdx: i64
    readonly metadata: ProtocolMetadata
}

export function readToClientInit(bc: bare.ByteCursor): ToClientInit {
    return {
        runnerId: readId(bc),
        lastEventIdx: bare.readI64(bc),
        metadata: readProtocolMetadata(bc),
    }
}

export function writeToClientInit(bc: bare.ByteCursor, x: ToClientInit): void {
    writeId(bc, x.runnerId)
    bare.writeI64(bc, x.lastEventIdx)
    writeProtocolMetadata(bc, x.metadata)
}

export type ToClientCommands = readonly CommandWrapper[]

export function readToClientCommands(bc: bare.ByteCursor): ToClientCommands {
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

export function writeToClientCommands(bc: bare.ByteCursor, x: ToClientCommands): void {
    bare.writeUintSafe(bc, x.length)
    for (let i = 0; i < x.length; i++) {
        writeCommandWrapper(bc, x[i])
    }
}

export type ToClientAckEvents = {
    readonly lastEventIdx: i64
}

export function readToClientAckEvents(bc: bare.ByteCursor): ToClientAckEvents {
    return {
        lastEventIdx: bare.readI64(bc),
    }
}

export function writeToClientAckEvents(bc: bare.ByteCursor, x: ToClientAckEvents): void {
    bare.writeI64(bc, x.lastEventIdx)
}

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

function read17(bc: bare.ByteCursor): readonly KvMetadata[] {
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

function write17(bc: bare.ByteCursor, x: readonly KvMetadata[]): void {
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
        keys: read13(bc),
        values: read16(bc),
        metadata: read17(bc),
    }
}

export function writeKvGetResponse(bc: bare.ByteCursor, x: KvGetResponse): void {
    write13(bc, x.keys)
    write16(bc, x.values)
    write17(bc, x.metadata)
}

export type KvListResponse = {
    readonly keys: readonly KvKey[]
    readonly values: readonly KvValue[]
    readonly metadata: readonly KvMetadata[]
}

export function readKvListResponse(bc: bare.ByteCursor): KvListResponse {
    return {
        keys: read13(bc),
        values: read16(bc),
        metadata: read17(bc),
    }
}

export function writeKvListResponse(bc: bare.ByteCursor, x: KvListResponse): void {
    write13(bc, x.keys)
    write16(bc, x.values)
    write17(bc, x.metadata)
}

export type KvPutResponse = null

export type KvDeleteResponse = null

export type KvDropResponse = null

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

export type ToClientKvResponse = {
    readonly requestId: u32
    readonly data: KvResponseData
}

export function readToClientKvResponse(bc: bare.ByteCursor): ToClientKvResponse {
    return {
        requestId: bare.readU32(bc),
        data: readKvResponseData(bc),
    }
}

export function writeToClientKvResponse(bc: bare.ByteCursor, x: ToClientKvResponse): void {
    bare.writeU32(bc, x.requestId)
    writeKvResponseData(bc, x.data)
}

export type ToClient =
    | { readonly tag: "ToClientInit"; readonly val: ToClientInit }
    | { readonly tag: "ToClientCommands"; readonly val: ToClientCommands }
    | { readonly tag: "ToClientAckEvents"; readonly val: ToClientAckEvents }
    | { readonly tag: "ToClientKvResponse"; readonly val: ToClientKvResponse }

export function readToClient(bc: bare.ByteCursor): ToClient {
    const offset = bc.offset
    const tag = bare.readU8(bc)
    switch (tag) {
        case 0:
            return { tag: "ToClientInit", val: readToClientInit(bc) }
        case 1:
            return { tag: "ToClientCommands", val: readToClientCommands(bc) }
        case 2:
            return { tag: "ToClientAckEvents", val: readToClientAckEvents(bc) }
        case 3:
            return { tag: "ToClientKvResponse", val: readToClientKvResponse(bc) }
        default: {
            bc.offset = offset
            throw new bare.BareError(offset, "invalid tag")
        }
    }
}

export function writeToClient(bc: bare.ByteCursor, x: ToClient): void {
    switch (x.tag) {
        case "ToClientInit": {
            bare.writeU8(bc, 0)
            writeToClientInit(bc, x.val)
            break
        }
        case "ToClientCommands": {
            bare.writeU8(bc, 1)
            writeToClientCommands(bc, x.val)
            break
        }
        case "ToClientAckEvents": {
            bare.writeU8(bc, 2)
            writeToClientAckEvents(bc, x.val)
            break
        }
        case "ToClientKvResponse": {
            bare.writeU8(bc, 3)
            writeToClientKvResponse(bc, x.val)
            break
        }
    }
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
