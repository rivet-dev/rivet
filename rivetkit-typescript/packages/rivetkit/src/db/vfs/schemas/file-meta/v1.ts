import * as bare from "@bare-ts/lib"

const config = /* @__PURE__ */ bare.Config({})

export type u64 = bigint

export type FileMeta = {
    readonly size: u64,
}

export function readFileMeta(bc: bare.ByteCursor): FileMeta {
    return {
        size: bare.readU64(bc),
    }
}

export function writeFileMeta(bc: bare.ByteCursor, x: FileMeta): void {
    bare.writeU64(bc, x.size)
}

export function encodeFileMeta(x: FileMeta): Uint8Array {
    const bc = new bare.ByteCursor(
        new Uint8Array(config.initialBufferLength),
        config
    )
    writeFileMeta(bc, x)
    return new Uint8Array(bc.view.buffer, bc.view.byteOffset, bc.offset)
}

export function decodeFileMeta(bytes: Uint8Array): FileMeta {
    const bc = new bare.ByteCursor(bytes, config)
    const result = readFileMeta(bc)
    if (bc.offset < bc.view.byteLength) {
        throw new bare.BareError(bc.offset, "remaining bytes")
    }
    return result
}
