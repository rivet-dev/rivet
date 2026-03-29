# VFS v2: Chunked Storage with Metadata Separation

## Status: Draft

## Codebase Location

This work happens in **secure-exec** (`~/secure-exec-1`). The VirtualFileSystem interface, kernel, mount table, and all core VFS implementations live there. Agent-os consumes secure-exec as a dependency.

### Files to modify

- `packages/core/src/kernel/vfs.ts` -- VirtualFileSystem interface (add pwrite, fsync, copy, readDirStat)
- `packages/core/src/kernel/kernel.ts` -- kernel I/O paths (delegate pwrite to VFS instead of read-modify-write; call fsync on last FD close)
- `packages/core/src/kernel/mount-table.ts` -- mount routing (add new VFS methods)

### Files to delete

- `packages/core/src/shared/in-memory-fs.ts` -- replaced by ChunkedVFS(InMemoryMetadataStore + InMemoryBlockStore)
- `packages/core/src/kernel/inode-table.ts` -- inode management moves into FsMetadataStore

### New files in secure-exec

- `packages/core/src/vfs/types.ts` -- FsMetadataStore and FsBlockStore interfaces
- `packages/core/src/vfs/chunked-vfs.ts` -- ChunkedVFS composition
- `packages/core/src/vfs/memory-metadata.ts` -- InMemoryMetadataStore (pure JS Map-based)
- `packages/core/src/vfs/memory-block-store.ts` -- InMemoryBlockStore
- `packages/core/src/vfs/sqlite-metadata.ts` -- SqliteMetadataStore
- `packages/core/src/vfs/host-block-store.ts` -- HostBlockStore (stores blocks as files on host FS)
- `packages/core/src/test/vfs-conformance.ts` -- shared VFS conformance test suite (exported)
- `packages/core/src/test/block-store-conformance.ts` -- shared block store test suite (exported)
- `packages/core/src/test/metadata-store-conformance.ts` -- shared metadata store test suite (exported)

### Agent-os packages to delete

- `agent-os/packages/fs-s3/` -- rewritten as a thin S3BlockStore
- `agent-os/packages/fs-sqlite/` -- deleted (metadata lives in secure-exec)
- `agent-os/packages/fs-postgres/` -- deleted

### Agent-os new packages

- `agent-os/packages/fs-s3/` -- S3BlockStore implementation of FsBlockStore

### What stays (not rewritten)

- `agent-os/packages/core/src/backends/host-dir-backend.ts` -- pass-through VFS for mounting host directories read-only. Implements VirtualFileSystem directly.
- `packages/core/src/kernel/device-backend.ts` -- /dev mount (kernel-internal)
- `packages/core/src/kernel/proc-backend.ts` -- /proc mount (kernel-internal)
- `packages/core/src/kernel/mount-table.ts` -- mount routing layer
- `packages/core/src/kernel/permissions.ts` -- permission wrapper

### Rivet actor integration

The Rivet actor integration (`rivetkit-typescript/packages/rivetkit/src/agent-os/`) should use ChunkedVFS(InMemoryMetadataStore + InMemoryBlockStore) for now. TODO: reimplement with a proper persistent backend (e.g., actor KV-backed metadata + actor storage-backed blocks).

---

## Drivers

We are deleting ALL existing VFS storage backends and implementing exactly three chunked drivers, plus keeping one pass-through:

| Driver | Metadata | Blocks | Use Case |
|--------|----------|--------|----------|
| **In-memory** | InMemoryMetadataStore | InMemoryBlockStore | Ephemeral VMs, tests, Rivet actors (for now) |
| **Host filesystem** | SqliteMetadataStore | HostBlockStore | Local persistent dev environments |
| **SQLite + S3** | SqliteMetadataStore | S3BlockStore | Cloud/remote persistent storage |
| **Host directory** (pass-through) | N/A | N/A | Read-only host mounts into VM |

All three chunked drivers compose via ChunkedVFS and must pass the same VFS conformance test suite exported by secure-exec.

---

## Overview

Replace the current monolithic VFS backends (where each backend independently implements directory trees, permissions, symlinks, path resolution, AND file content storage) with a layered architecture:

1. **VirtualFileSystem** -- the universal interface the kernel talks to. Extended with pwrite, fsync, copy, readDirStat.
2. **FsMetadataStore** -- owns the directory tree, inodes, permissions, symlinks, chunk mapping. No file content. Has transaction support for atomic multi-step operations.
3. **FsBlockStore** -- dumb key-value byte store. No filesystem concepts.
4. **ChunkedVFS** -- composes a metadata store + block store into a VirtualFileSystem. Owns chunk math, tiered storage, concurrency control, and optional write buffering.
5. **Pass-through backends** -- implement VirtualFileSystem directly (HostDirBackend).

Reference: JuiceFS architecture (https://juicefs.com/docs/community/architecture/), adapted for single-client VM workloads.

## Design Principles

- The kernel only talks to `VirtualFileSystem`. It never knows about chunks, blocks, or metadata stores.
- `VirtualFileSystem` mirrors POSIX syscalls with the FD layer stripped off. The kernel owns FDs, cursors, and open file state. The VFS owns storage.
- `ChunkedVFS` is one implementation of `VirtualFileSystem`. Pass-through backends are others.
- Metadata and data are separated. Metadata operations (stat, readdir, exists, rename) are frequent, small, and latency-sensitive. Data operations (read/write content) are larger and tolerate higher latency.
- All multi-step metadata mutations happen inside transactions. No partial state on crash.
- ChunkedVFS uses a per-inode async mutex to prevent interleaved read-modify-write corruption from concurrent async operations.

---

## VirtualFileSystem Interface

The universal contract. What the kernel and mount table talk to.

```typescript
interface VirtualFileSystem {
    // -- Core I/O --
    readFile(path: string): Promise<Uint8Array>;
    readTextFile(path: string): Promise<string>;
    writeFile(path: string, content: string | Uint8Array): Promise<void>;
    exists(path: string): Promise<boolean>;
    stat(path: string): Promise<VirtualStat>;

    // -- Positional I/O --

    /** Read a byte range without loading the entire file. */
    pread(path: string, offset: number, length: number): Promise<Uint8Array>;

    /** Write bytes at a specific offset without replacing the entire file. */
    pwrite(path: string, offset: number, data: Uint8Array): Promise<void>;

    truncate(path: string, length: number): Promise<void>;

    // -- Flush --

    /**
     * Flush any buffered writes for the given path to durable storage.
     * Optional. Backends without write buffering leave this undefined.
     * The kernel calls this on the fsync syscall and when the last FD
     * for a file is closed.
     */
    fsync?(path: string): Promise<void>;

    // -- Directory operations --
    readDir(path: string): Promise<string[]>;
    readDirWithTypes(path: string): Promise<VirtualDirEntry[]>;
    createDir(path: string): Promise<void>;
    mkdir(path: string, options?: { recursive?: boolean }): Promise<void>;
    removeDir(path: string): Promise<void>;

    /** Combined readdir + stat. Avoids N+1 queries. Optional. */
    readDirStat?(path: string): Promise<VirtualDirStatEntry[]>;

    // -- Path operations --
    rename(oldPath: string, newPath: string): Promise<void>;
    removeFile(path: string): Promise<void>;
    realpath(path: string): Promise<string>;

    /**
     * Intra-mount file copy. Avoids downloading + re-uploading data.
     * Used by the kernel for explicit copy operations.
     * Cross-mount rename returns EXDEV (matching Linux behavior);
     * the application is responsible for copy+delete if desired.
     * Optional. If not implemented, callers fall back to readFile + writeFile.
     */
    copy?(srcPath: string, dstPath: string): Promise<void>;

    // -- Symlinks & links --
    symlink(target: string, linkPath: string): Promise<void>;
    readlink(path: string): Promise<string>;
    lstat(path: string): Promise<VirtualStat>;
    link(oldPath: string, newPath: string): Promise<void>;

    // -- Permissions & metadata --
    chmod(path: string, mode: number): Promise<void>;
    chown(path: string, uid: number, gid: number): Promise<void>;
    utimes(path: string, atime: number, mtime: number): Promise<void>;
}

interface VirtualStat {
    mode: number;
    size: number;
    isDirectory: boolean;
    isSymbolicLink: boolean;
    atimeMs: number;
    mtimeMs: number;
    ctimeMs: number;
    birthtimeMs: number;
    ino: number;
    nlink: number;
    uid: number;
    gid: number;
}

interface VirtualDirEntry {
    name: string;
    isDirectory: boolean;
    isSymbolicLink?: boolean;
    ino?: number;
}

interface VirtualDirStatEntry extends VirtualDirEntry {
    stat: VirtualStat;
}
```

### Kernel integration

- The kernel delegates `pwrite` directly to `vfs.pwrite()` instead of doing read-modify-write. This is the critical change that makes chunked storage useful.
- The kernel calls `vfs.fsync?.(description.path)` on the fsync syscall and when the last FD referencing a FileDescription is closed (refCount reaches 0 in `releaseDescriptionInode`).
- Cross-mount `rename` returns EXDEV (matching Linux). The application handles copy+delete.
- The existing InMemoryFileSystem-specific kernel fast paths (`readFileByInode`, `preadByInode`, `writeFileByInode`, `statByInode`) are removed. All I/O goes through the VFS interface. The new InMemoryMetadataStore uses Map lookups which are fast enough for single-client VMs.
- `readDirStat` is called when the kernel needs directory listing + stat info. Falls back to readDir + individual stat calls if not implemented.
- `stat` follows symlinks and returns the target's metadata. `lstat` does not follow symlinks and returns `isSymbolicLink: true` for symlink inodes. ChunkedVFS implements this by using `resolvePath` (follows symlinks) for stat and `resolveParentPath` + direct inode lookup for lstat.

---

## FsMetadataStore Interface

Owns the filesystem tree, inode metadata, and chunk mapping. No file content. All path resolution happens here.

```typescript
interface FsMetadataStore {
    // -- Transactions --

    /**
     * Execute a callback atomically. All metadata mutations within
     * the callback either fully commit or fully roll back.
     * SQLite: wraps in BEGIN/COMMIT.
     * InMemory: just calls the callback (single-threaded JS, no rollback needed).
     */
    transaction<T>(fn: () => Promise<T>): Promise<T>;

    // -- Inode lifecycle --

    /** Create a new inode. Returns the allocated inode number. */
    createInode(attrs: CreateInodeAttrs): Promise<number>;

    /** Get inode metadata by number. Returns null if not found. */
    getInode(ino: number): Promise<InodeMeta | null>;

    /** Update inode metadata fields (partial update). */
    updateInode(ino: number, updates: Partial<InodeMeta>): Promise<void>;

    /** Delete an inode and all associated data (chunk map, symlink target). */
    deleteInode(ino: number): Promise<void>;

    // -- Directory entries --

    /** Look up a child name in a directory. Returns child ino or null. */
    lookup(parentIno: number, name: string): Promise<number | null>;

    /** Create a directory entry. Throws EEXIST if name already exists. */
    createDentry(parentIno: number, name: string, childIno: number, type: InodeType): Promise<void>;

    /** Remove a directory entry. Does NOT delete the child inode. */
    removeDentry(parentIno: number, name: string): Promise<void>;

    /** List all entries in a directory. */
    listDir(parentIno: number): Promise<DentryInfo[]>;

    /**
     * List all entries with full inode metadata (avoids N+1).
     * SQLite: single JOIN query. InMemory: iterate + Map lookup.
     */
    listDirWithStats(parentIno: number): Promise<DentryStatInfo[]>;

    /**
     * Move a directory entry. Atomic: removes from src parent,
     * adds to dst parent. Handles same-parent rename.
     */
    renameDentry(
        srcParentIno: number, srcName: string,
        dstParentIno: number, dstName: string,
    ): Promise<void>;

    // -- Path resolution --

    /**
     * Walk the dentry tree from root, following symlinks.
     * Returns the resolved inode number.
     * Throws ENOENT if any component does not exist.
     * Throws ELOOP if symlink depth exceeds 40 (SYMLOOP_MAX).
     *
     * Implementation note: metadata stores should optimize this
     * internally (caching, recursive CTEs, etc.) to avoid
     * O(depth) round-trips per call.
     */
    resolvePath(path: string): Promise<number>;

    /**
     * Resolve all intermediate path components but NOT the final one.
     * Returns the parent inode and the final component name.
     * Used for lstat, readlink, unlink, and creating new entries.
     * Throws ENOENT if any intermediate component does not exist.
     */
    resolveParentPath(path: string): Promise<{ parentIno: number; name: string }>;

    // -- Symlinks --

    /** Get the symlink target for a symlink inode. */
    readSymlink(ino: number): Promise<string>;

    // -- Chunk mapping --

    /** Get the block store key for a chunk. Null = sparse hole. */
    getChunkKey(ino: number, chunkIndex: number): Promise<string | null>;

    /** Set the block store key for a chunk. Creates or updates. */
    setChunkKey(ino: number, chunkIndex: number, key: string): Promise<void>;

    /** Get all chunk keys for a file, ordered by chunk index. */
    getAllChunkKeys(ino: number): Promise<{ chunkIndex: number; key: string }[]>;

    /** Delete all chunk mappings for an inode. Returns the deleted keys. */
    deleteAllChunks(ino: number): Promise<string[]>;

    /**
     * Delete chunk mappings for indices >= startIndex.
     * Returns the deleted keys. Used by truncate.
     */
    deleteChunksFrom(ino: number, startIndex: number): Promise<string[]>;
}

// -- Types --

type InodeType = 'file' | 'directory' | 'symlink';

interface CreateInodeAttrs {
    type: InodeType;
    mode: number;
    uid: number;
    gid: number;
    /** Required for symlinks. */
    symlinkTarget?: string;
}

interface InodeMeta {
    ino: number;
    type: InodeType;
    mode: number;
    uid: number;
    gid: number;
    size: number;
    nlink: number;
    atimeMs: number;
    mtimeMs: number;
    ctimeMs: number;
    birthtimeMs: number;
    /**
     * 'inline': content stored in inlineContent (small files).
     * 'chunked': content stored as blocks in the block store.
     */
    storageMode: 'inline' | 'chunked';
    /** Inline content for small files. Null if chunked. */
    inlineContent: Uint8Array | null;
}

interface DentryInfo {
    name: string;
    ino: number;
    type: InodeType;
}

interface DentryStatInfo extends DentryInfo {
    stat: InodeMeta;
}
```

### Path resolution performance

The `resolvePath` and `resolveParentPath` methods should be optimized by the metadata store implementation:

- **InMemoryMetadataStore**: Direct Map lookups per component. Fast (nanoseconds per lookup). Can optionally cache full path-to-inode mappings with invalidation on rename/unlink.
- **SqliteMetadataStore**: Can use a recursive CTE or a loop of `SELECT child_ino FROM dentries WHERE parent_ino = ? AND name = ?` queries. Each query is fast with the PRIMARY KEY index. For deep paths, consider caching frequently-accessed directory inodes.

The interface does not dictate the implementation strategy. Stores that see high path-resolution traffic should index appropriately.

---

## FsBlockStore Interface

Dumb key-value byte store. Knows nothing about files, directories, or inodes.

```typescript
interface FsBlockStore {
    /** Read an entire block. Throws if key not found. */
    read(key: string): Promise<Uint8Array>;

    /** Read a byte range within a block. Throws if key not found. */
    readRange(key: string, offset: number, length: number): Promise<Uint8Array>;

    /** Write a block (creates or overwrites). */
    write(key: string, data: Uint8Array): Promise<void>;

    /** Delete a block. No-op if key does not exist. */
    delete(key: string): Promise<void>;

    /** Delete multiple blocks. No-op for keys that don't exist. */
    deleteMany(keys: string[]): Promise<void>;

    /**
     * Server-side copy. Optional.
     * If not implemented, callers fall back to read + write.
     */
    copy?(srcKey: string, dstKey: string): Promise<void>;
}
```

### Error contracts

- `read` and `readRange`: throw `KernelError("ENOENT", ...)` if key not found.
- `readRange` with offset beyond block size: return available bytes (short read), not an error.
- `write`: overwrite if key exists. No size limits enforced at interface level (implementations may reject).
- `delete` and `deleteMany`: no-op for non-existent keys. Never throw for missing keys.
- `copy`: throw `KernelError("ENOENT", ...)` if source key not found.

---

## ChunkedVFS

Composes `FsMetadataStore` + `FsBlockStore` into a `VirtualFileSystem`.

### Configuration

```typescript
interface ChunkedVfsOptions {
    metadata: FsMetadataStore;
    blocks: FsBlockStore;

    /** Chunk size in bytes. Default: 4 * 1024 * 1024 (4 MB). */
    chunkSize?: number;

    /**
     * Max file size for inline storage in the metadata store.
     * Files at or below this size are stored directly in InodeMeta.inlineContent.
     * Default: 65536 (64 KB).
     */
    inlineThreshold?: number;

    /**
     * Enable write buffering. When true, pwrite buffers dirty chunks
     * in memory and flushes to the block store on fsync or auto-flush.
     * When false, every pwrite immediately writes to the block store.
     * Default: false (unbuffered, correct but slower for remote block stores).
     */
    writeBuffering?: boolean;

    /**
     * Auto-flush interval in ms. Only applies when writeBuffering is true.
     * Dirty chunks are flushed to the block store on this interval.
     * Default: 1000 (1 second).
     */
    autoFlushIntervalMs?: number;

    /**
     * Enable file versioning. When true, each write creates a new block
     * key (never overwrites), and version snapshots can be created.
     * When false, writes overwrite the same block key (simpler, no GC needed).
     * Default: false.
     */
    versioning?: boolean;
}
```

### Chunk addressing

```
chunk_index     = floor(byte_offset / chunkSize)
offset_in_chunk = byte_offset % chunkSize
```

Block key format depends on versioning mode:
- **Versioning disabled**: `{ino}/{chunkIndex}` -- overwrites on each write. No orphaned blocks.
- **Versioning enabled**: `{ino}/{chunkIndex}/{randomId}` -- each write creates a new key. Old keys preserved for version snapshots. Requires GC to clean up orphaned blocks.

### Tiered storage

| File size | storageMode | Where data lives |
|-----------|------------|------------------|
| <= inlineThreshold | `inline` | `InodeMeta.inlineContent` (metadata store) |
| > inlineThreshold | `chunked` | Block store, one key per chunk |

Promotion (inline to chunked) and demotion (chunked to inline) happen automatically when a file crosses the threshold via pwrite, truncate, or writeFile. These transitions happen inside a metadata transaction.

### Concurrency: per-inode mutex

JavaScript is single-threaded but async. Two concurrent pwrites to the same inode can interleave at `await` points, causing a lost-write race:

```
pwrite A: read chunk -> [yields] -> pwrite B: read chunk -> B writes -> A writes (B's data lost)
```

ChunkedVFS maintains a `Map<number, Promise<void>>` per-inode mutex. All operations that modify an inode (pwrite, writeFile, truncate, removeFile, rename) acquire the mutex before proceeding. Read-only operations (pread, readFile, stat) do not need the mutex (they see a consistent snapshot even if a write is in progress, because each step produces valid intermediate state).

```typescript
class InodeMutex {
    private locks = new Map<number, Promise<void>>();

    async acquire(ino: number): Promise<() => void> {
        while (this.locks.has(ino)) {
            await this.locks.get(ino);
        }
        let release!: () => void;
        this.locks.set(ino, new Promise(r => { release = r; }));
        return () => { this.locks.delete(ino); release(); };
    }
}
```

### Write buffering (optional, for remote block stores)

When `writeBuffering` is enabled, ChunkedVFS maintains a dirty chunk buffer:

```typescript
// Internal state per active inode
interface WriteBuffer {
    ino: number;
    dirtyChunks: Map<number, Uint8Array>; // chunkIndex -> modified chunk data
    pendingSize: number; // total bytes in dirty chunks
}
```

- **pwrite**: resolves path to inode, reads chunk into buffer if not cached, modifies bytes, marks chunk dirty. Does NOT write to block store.
- **pread**: checks dirty buffer first. If the requested range overlaps a dirty chunk, serves from buffer. Otherwise reads from block store.
- **fsync(path)**: resolves path to inode, writes all dirty chunks to block store, updates chunk keys in metadata, clears dirty state.
- **Auto-flush timer**: periodically flushes ALL dirty chunks across all inodes. Configurable interval.
- **readFile/stat**: checks dirty buffer for accurate size and content.

When `writeBuffering` is disabled (default), all writes go directly to the block store. fsync is a no-op.

---

## Data Flows

### pread(path, offset, length)

```
1. resolvePath(path) -> ino
2. getInode(ino) -> meta
3. Clamp length: if offset >= meta.size, return empty Uint8Array
   If offset + length > meta.size, clamp to meta.size - offset
4. If length == 0: return empty Uint8Array
5. If meta.storageMode == 'inline':
     return meta.inlineContent.slice(offset, offset + length)
6. If meta.storageMode == 'chunked':
     startChunk = floor(offset / chunkSize)
     endChunk   = floor((offset + length - 1) / chunkSize)
     For each chunk in [startChunk, endChunk]:
       If writeBuffering && chunk in dirtyBuffer:
         Use buffered data
       Else:
         key = getChunkKey(ino, chunk)
         If key is null -> zeros (sparse hole)
         Else: blocks.readRange(key, rangeStart, rangeEnd - rangeStart)
     Concatenate and return
```

### pwrite(path, offset, data)

```
1. resolvePath(path) -> ino
2. Acquire inode mutex
3. getInode(ino) -> meta
4. newSize = max(meta.size, offset + data.length)
5. If meta.storageMode == 'inline':
     If newSize <= inlineThreshold:
       Modify inlineContent in place (extend with zeros if needed)
       updateInode(ino, { inlineContent, size: newSize, mtimeMs: now })
       Release mutex, return
     Else:
       Promote to chunked (inside transaction):
         Write existing inlineContent as chunk(s)
         Set storageMode = 'chunked', clear inlineContent
       Fall through to chunked pwrite
6. If meta.storageMode == 'chunked':
     For each affected chunk:
       key = getChunkKey(ino, chunkIndex)
       If writeBuffering:
         Load chunk into dirty buffer (from block store if not cached)
         Modify bytes in buffer
       Else:
         If key exists: existing = blocks.read(key)
         Else: existing = new Uint8Array(chunkSize) (zeros)
         Modify bytes in existing
         newKey = generateBlockKey(ino, chunkIndex)
         blocks.write(newKey, modifiedChunk)
         setChunkKey(ino, chunkIndex, newKey)
     updateInode(ino, { size: newSize, mtimeMs: now })
7. Release mutex
```

### writeFile(path, content)

```
1. resolveParentPath(path) -> { parentIno, name }
   On ENOENT for any intermediate component:
     Recursively create parent directories (mkdir -p)
2. Acquire inode mutex (if existing file)
3. lookup(parentIno, name) -> existingIno or null
4. transaction {
     If existingIno:
       getInode(existingIno) -> meta
       If meta was chunked: delete old chunks from block store
       Clear existing data
     Else:
       createInode({ type: 'file', mode: 0o644, uid: 0, gid: 0 }) -> newIno
       createDentry(parentIno, name, newIno, 'file')
       existingIno = newIno

     If content.length <= inlineThreshold:
       updateInode(existingIno, {
         storageMode: 'inline', inlineContent: content,
         size: content.length, mtimeMs: now
       })
     Else:
       Split content into chunkSize pieces
       For each piece:
         key = generateBlockKey(existingIno, chunkIndex)
         blocks.write(key, piece)
         setChunkKey(existingIno, chunkIndex, key)
       updateInode(existingIno, {
         storageMode: 'chunked', inlineContent: null,
         size: content.length, mtimeMs: now
       })
   }
5. Release mutex
```

### readFile(path)

```
1. resolvePath(path) -> ino
2. getInode(ino) -> meta
3. If meta.storageMode == 'inline':
     return meta.inlineContent (or empty Uint8Array if null/size 0)
4. If meta.storageMode == 'chunked':
     getAllChunkKeys(ino) -> chunks[]
     result = new Uint8Array(meta.size)
     For each chunk in chunks:
       If writeBuffering && chunk in dirtyBuffer: use buffered data
       Else: data = blocks.read(chunk.key)
       Copy data into result at correct offset
     return result
```

### removeFile(path)

```
1. resolveParentPath(path) -> { parentIno, name }
2. lookup(parentIno, name) -> childIno (throw ENOENT if null)
3. getInode(childIno) -> meta
4. If meta.type == 'directory': throw EISDIR
5. Acquire inode mutex
6. transaction {
     removeDentry(parentIno, name)
     newNlink = meta.nlink - 1
     If newNlink == 0:
       If meta.storageMode == 'chunked':
         keys = deleteAllChunks(childIno)
         blocks.deleteMany(keys)
       deleteInode(childIno)
     Else:
       updateInode(childIno, { nlink: newNlink, ctimeMs: now })
   }
7. Release mutex
```

### truncate(path, newLength)

```
1. resolvePath(path) -> ino
2. Acquire inode mutex
3. getInode(ino) -> meta
4. If newLength == meta.size: no-op, return
5. If shrinking (newLength < meta.size):
     If meta.storageMode == 'inline':
       Slice inlineContent to newLength
       If newLength == 0: set inlineContent to empty
     If meta.storageMode == 'chunked':
       lastChunkIndex = floor((newLength - 1) / chunkSize) if newLength > 0 else -1
       Delete chunks beyond lastChunkIndex:
         keys = deleteChunksFrom(ino, lastChunkIndex + 1)
         blocks.deleteMany(keys)
       If last remaining chunk is partial:
         Read it, slice to correct length, write back
       If newLength <= inlineThreshold:
         Demote to inline: read remaining data, store as inlineContent,
         delete remaining chunks from block store
6. If growing (newLength > meta.size):
     If meta.storageMode == 'inline':
       If newLength <= inlineThreshold:
         Zero-extend inlineContent
       Else:
         Promote to chunked: write existing content as chunks, zero-extend
     If meta.storageMode == 'chunked':
       Update size only (sparse: unwritten regions read as zeros)
7. updateInode(ino, { size: newLength, mtimeMs: now, ctimeMs: now })
8. Release mutex
```

### copy(srcPath, dstPath)

```
1. resolvePath(srcPath) -> srcIno
2. getInode(srcIno) -> srcMeta
3. resolveParentPath(dstPath) -> { parentIno, name }
4. transaction {
     createInode({ type: 'file', mode: srcMeta.mode, ... }) -> dstIno
     createDentry(parentIno, name, dstIno, 'file')
     If srcMeta.storageMode == 'inline':
       Copy inlineContent to new inode
     If srcMeta.storageMode == 'chunked':
       For each chunk key in src:
         newKey = generateBlockKey(dstIno, chunkIndex)
         If blocks.copy exists:
           blocks.copy(srcKey, newKey)  // S3 server-side copy
         Else:
           data = blocks.read(srcKey)
           blocks.write(newKey, data)
         setChunkKey(dstIno, chunkIndex, newKey)
   }
```

### rename(oldPath, newPath)

```
1. resolveParentPath(oldPath) -> { parentIno: srcParent, name: srcName }
2. resolveParentPath(newPath) -> { parentIno: dstParent, name: dstName }
3. lookup(srcParent, srcName) -> srcIno (throw ENOENT if null)
4. lookup(dstParent, dstName) -> existingDstIno or null
5. transaction {
     If existingDstIno:
       getInode(existingDstIno) -> dstMeta
       If dstMeta is file: removeFile logic (decrement nlink, delete if 0)
       If dstMeta is directory and not empty: throw ENOTEMPTY
     renameDentry(srcParent, srcName, dstParent, dstName)
   }
```

### fsync(path)

```
1. If not writeBuffering: no-op, return
2. resolvePath(path) -> ino
3. Acquire inode mutex
4. If ino has dirty chunks in buffer:
     For each dirty chunk:
       newKey = generateBlockKey(ino, chunkIndex)
       blocks.write(newKey, chunkData)
       setChunkKey(ino, chunkIndex, newKey)
     updateInode(ino, { size: currentSize, mtimeMs: now })
     Clear dirty state for this inode
5. Release mutex
```

If path resolution fails (e.g., file was unlinked), fsync silently returns. The auto-flush timer will flush any remaining dirty data by iterating all dirty inodes directly (not by path).

---

## Versioning

Versioning is **optional** and **per-ChunkedVFS-instance**. When enabled:

- Each write creates a new unique block key (never overwrites old keys).
- Version snapshots can be created explicitly via `createVersion()`.
- Old block keys are preserved until version pruning or GC.

When disabled (default):
- Writes overwrite the same block key. Simpler, no orphaned blocks, no GC needed.

### Versioning API (on ChunkedVFS, not on VFS interface)

```typescript
interface ChunkedVfsVersioning {
    /** Snapshot current state. Returns version number. */
    createVersion(path: string): Promise<number>;

    /** List all versions of a file, newest first. */
    listVersions(path: string): Promise<VersionInfo[]>;

    /** Restore file to a previous version. */
    restoreVersion(path: string, version: number): Promise<void>;

    /**
     * Prune old versions according to policy.
     * Returns count of versions pruned.
     * Orphaned block cleanup depends on the retention policy.
     */
    pruneVersions(path: string, policy: RetentionPolicy): Promise<number>;
}

interface VersionInfo {
    version: number;
    size: number;
    createdAt: number;
}

type RetentionPolicy =
    /** Keep the N most recent versions. Delete the rest immediately. */
    | { type: 'count'; keep: number }
    /** Keep versions newer than maxAgeMs. Delete older immediately. */
    | { type: 'age'; maxAgeMs: number }
    /**
     * Mark old metadata as pruned but do NOT delete blocks.
     * Used with block stores that have their own TTL/lifecycle
     * (e.g., S3 lifecycle rules). The block store handles cleanup.
     */
    | { type: 'deferred' };
```

### Versioning metadata (on FsMetadataStore, optional)

Metadata stores that support versioning implement these additional methods:

```typescript
interface FsMetadataStoreVersioning {
    /** Snapshot current chunk map + size. Returns version number. */
    createVersion(ino: number): Promise<number>;

    /** Get version info. */
    getVersion(ino: number, version: number): Promise<VersionMeta | null>;

    /** List versions, newest first. */
    listVersions(ino: number): Promise<VersionMeta[]>;

    /** Get chunk map for a specific version. */
    getVersionChunkMap(ino: number, version: number): Promise<{ chunkIndex: number; key: string }[]>;

    /**
     * Delete version records. Returns block keys that are no longer
     * referenced by ANY version or the current chunk map.
     */
    deleteVersions(ino: number, versions: number[]): Promise<string[]>;

    /** Restore current chunk map to match a version. */
    restoreVersion(ino: number, version: number): Promise<void>;
}

interface VersionMeta {
    version: number;
    size: number;
    createdAt: number;
    storageMode: 'inline' | 'chunked';
    inlineContent: Uint8Array | null;
}
```

---

## SQLite Metadata Schema

```sql
CREATE TABLE inodes (
    ino              INTEGER PRIMARY KEY AUTOINCREMENT,
    type             TEXT NOT NULL CHECK(type IN ('file', 'directory', 'symlink')),
    mode             INTEGER NOT NULL,
    uid              INTEGER NOT NULL DEFAULT 0,
    gid              INTEGER NOT NULL DEFAULT 0,
    size             INTEGER NOT NULL DEFAULT 0,
    nlink            INTEGER NOT NULL DEFAULT 1,
    atime_ms         INTEGER NOT NULL,
    mtime_ms         INTEGER NOT NULL,
    ctime_ms         INTEGER NOT NULL,
    birthtime_ms     INTEGER NOT NULL,
    storage_mode     TEXT NOT NULL DEFAULT 'inline' CHECK(storage_mode IN ('inline', 'chunked')),
    inline_content   BLOB
);

CREATE TABLE dentries (
    parent_ino  INTEGER NOT NULL,
    name        TEXT NOT NULL,
    child_ino   INTEGER NOT NULL,
    child_type  TEXT NOT NULL,
    PRIMARY KEY (parent_ino, name),
    FOREIGN KEY (parent_ino) REFERENCES inodes(ino),
    FOREIGN KEY (child_ino) REFERENCES inodes(ino)
);
CREATE INDEX idx_dentries_child ON dentries(child_ino);

CREATE TABLE symlinks (
    ino     INTEGER PRIMARY KEY,
    target  TEXT NOT NULL,
    FOREIGN KEY (ino) REFERENCES inodes(ino)
);

CREATE TABLE chunks (
    ino          INTEGER NOT NULL,
    chunk_index  INTEGER NOT NULL,
    block_key    TEXT NOT NULL,
    PRIMARY KEY (ino, chunk_index),
    FOREIGN KEY (ino) REFERENCES inodes(ino)
);

-- Only created when versioning is enabled.
CREATE TABLE versions (
    ino          INTEGER NOT NULL,
    version      INTEGER NOT NULL,
    size         INTEGER NOT NULL,
    created_at   INTEGER NOT NULL,
    storage_mode TEXT NOT NULL,
    inline_content BLOB,
    -- JSON array of {chunkIndex, blockKey}.
    chunk_map    TEXT,
    PRIMARY KEY (ino, version),
    FOREIGN KEY (ino) REFERENCES inodes(ino)
);
```

Root inode (ino=1, type='directory') is created at format time.

---

## Block Store Implementations

### InMemoryBlockStore

Pure JavaScript Map-based. For ephemeral VMs and tests.

```typescript
class InMemoryBlockStore implements FsBlockStore {
    private store = new Map<string, Uint8Array>();

    async read(key) { /* Map.get, throw ENOENT if missing */ }
    async readRange(key, offset, length) { /* read + slice */ }
    async write(key, data) { /* Map.set */ }
    async delete(key) { /* Map.delete */ }
    async deleteMany(keys) { /* loop Map.delete */ }
    async copy(src, dst) { /* Map.get + Map.set with new Uint8Array copy */ }
}
```

### HostBlockStore

Stores blocks as files on the host filesystem. For local persistent storage.

```typescript
class HostBlockStore implements FsBlockStore {
    constructor(private baseDir: string) {}

    // key "42/3" -> file at "{baseDir}/42/3"
    // Uses node:fs/promises for all I/O.
    // readRange uses fs.open + handle.read with position param.
    // Directories created on demand for key prefixes.
}
```

### S3BlockStore

Stores blocks in S3-compatible object storage.

```typescript
class S3BlockStore implements FsBlockStore {
    constructor(private options: S3BlockStoreOptions) {}

    // key -> S3 object at "{prefix}/blocks/{key}"
    // read: GetObjectCommand
    // readRange: GetObjectCommand with Range header
    // write: PutObjectCommand
    // delete: DeleteObjectCommand
    // deleteMany: DeleteObjectsCommand (batched in groups of 1000)
    // copy: CopyObjectCommand (server-side, no data transfer)
}
```

S3-specific considerations:
- `deleteMany` batches internally (S3 limit: 1000 per request). All batches are attempted; failures are collected and rethrown as a single error listing failed keys.
- For versioning with deferred retention: use S3 lifecycle rules to auto-delete objects with a specific tag or prefix after a TTL, instead of explicit deletion.

---

## Orphaned Block GC

Blocks can become orphaned if a crash occurs between writing a block and updating the metadata, or when using deferred version retention.

### GC strategy is driver-level

Different block stores have different cleanup mechanisms:

| Block Store | GC Strategy |
|-------------|-------------|
| **InMemoryBlockStore** | No GC needed. Data is ephemeral. |
| **HostBlockStore** | Periodic sweep: list all files in baseDir, compare with metadata store's referenced keys, delete unreferenced files. |
| **S3BlockStore** | Option A: Same sweep strategy. Option B: S3 lifecycle rules with TTL on objects. Tag new blocks as "active"; orphaned blocks lack the tag and expire. |

### GC interface (optional, on ChunkedVFS)

```typescript
interface ChunkedVfsGc {
    /**
     * Find and delete orphaned blocks.
     * Scans the block store for keys not referenced by any
     * current chunk mapping or version snapshot.
     * Returns count of blocks deleted.
     *
     * Expensive: lists ALL keys in the block store. Run periodically, not on every operation.
     */
    collectGarbage(): Promise<number>;
}
```

For block stores that use external TTL (like S3 lifecycle rules), GC is unnecessary. ChunkedVFS should not call `collectGarbage` automatically; it is the application's responsibility to schedule it.

---

## Kernel Changes

### 1. Delegate pwrite to VFS

Current kernel `fdPwrite` (kernel.ts:961-981) reads the entire file, patches bytes, and writes back. Change to:

```typescript
fdPwrite: async (pid, fd, data, offset) => {
    // ... existing FD validation, pipe/PTY rejection ...
    const vfs = this.mountTable; // or resolve mount for description.path
    await vfs.pwrite(entry.description.path, Number(offset), data);
    return data.length;
}
```

### 2. Call fsync on last FD close

In `releaseDescriptionInode` (kernel.ts:1533-1543), after decrementing open refs:

```typescript
private async releaseDescriptionInode(description: FileDescription): Promise<void> {
    // Flush buffered writes before releasing
    if (typeof this.mountTable.fsync === 'function') {
        try { await this.mountTable.fsync(description.path); } catch {}
    }
    // ... existing inode cleanup logic ...
}
```

Note: `releaseDescriptionInode` is currently synchronous (called from `fdClose`). It will need to become async, and `fdClose` will need to await it. This is a contained change since `fdClose` is already part of the async kernel interface.

### 3. Remove InMemoryFileSystem fast paths

Remove `readFileByInode`, `preadByInode`, `writeFileByInode`, `statByInode` fast paths from the kernel. All I/O goes through the VFS interface. The `rawInMemoryFs` field and `trackDescriptionInode`/`releaseDescriptionInode` inode-tracking logic are simplified or removed.

For unlinked-file-read (open a file, delete it, keep reading via FD): this is an edge case for v1. ChunkedVFS's metadata store keeps the inode alive (nlink=0 but not deleted) until a GC pass or explicit cleanup. The kernel can continue using `description.path` for I/O until the FD is closed, even though the dentry is removed, because ChunkedVFS's pread resolves the path at the inode level (the inode still exists even though its dentry is gone).

Actually, that won't work because `resolvePath` walks the dentry tree. If the dentry is removed, path resolution fails. For v1, accept this limitation: unlinked-file-read is not supported for ChunkedVFS backends. The InMemoryMetadataStore could special-case this by keeping a path alias for unlinked inodes, but this is deferred.

TODO: For full POSIX unlink-while-open support, the kernel should track open inodes and route I/O by inode number when the path is stale. This requires adding inode-based read/write methods to the VFS interface. Deferred to a future version.

### 4. Add pwrite and new methods to MountTable

The MountTable (which implements VirtualFileSystem) needs to delegate the new methods:

```typescript
// In MountTable:
async pwrite(path, offset, data) {
    const resolved = this.resolve(path);
    this.assertWritable(resolved.mount, path);
    return resolved.mount.fs.pwrite(resolved.relativePath, offset, data);
}

async fsync(path) {
    const resolved = this.resolve(path);
    return resolved.mount.fs.fsync?.(resolved.relativePath);
}

async copy(srcPath, dstPath) {
    const srcResolved = this.resolve(srcPath);
    const dstResolved = this.resolve(dstPath);
    if (srcResolved.mount !== dstResolved.mount) {
        throw new KernelError("EXDEV", `copy across mounts: ${srcPath} -> ${dstPath}`);
    }
    this.assertWritable(srcResolved.mount, dstPath);
    if (srcResolved.mount.fs.copy) {
        return srcResolved.mount.fs.copy(srcResolved.relativePath, dstResolved.relativePath);
    }
    // Fallback: readFile + writeFile
    const data = await srcResolved.mount.fs.readFile(srcResolved.relativePath);
    await dstResolved.mount.fs.writeFile(dstResolved.relativePath, data);
}
```

---

## Testing Strategy

All test suites are exported from secure-exec so that external packages (agent-os block stores, third-party drivers) can run them.

### Layer 1: VFS Conformance Tests

**Location**: `secure-exec-1/packages/core/src/test/vfs-conformance.ts`

Tests the `VirtualFileSystem` interface. Every driver must pass these.

```typescript
interface VfsConformanceConfig {
    name: string;
    createFs: () => Promise<VirtualFileSystem> | VirtualFileSystem;
    cleanup?: () => Promise<void>;
    capabilities: {
        symlinks: boolean;
        hardLinks: boolean;
        permissions: boolean;
        utimes: boolean;
        truncate: boolean;
        pread: boolean;
        pwrite: boolean;
        mkdir: boolean;
        removeDir: boolean;
        fsync: boolean;
        copy: boolean;
        readDirStat: boolean;
    };
}

function defineVfsConformanceTests(config: VfsConformanceConfig): void;
```

**Drivers that register:**

```typescript
// In-memory ChunkedVFS
defineVfsConformanceTests({
    name: 'ChunkedVFS (InMemory + InMemory)',
    createFs: () => createChunkedVfs({
        metadata: new InMemoryMetadataStore(),
        blocks: new InMemoryBlockStore(),
    }),
    capabilities: { symlinks: true, hardLinks: true, permissions: true,
        pwrite: true, fsync: false, copy: true, readDirStat: true, ... },
});

// Host filesystem ChunkedVFS
defineVfsConformanceTests({
    name: 'ChunkedVFS (SQLite + HostBlockStore)',
    createFs: () => createChunkedVfs({
        metadata: new SqliteMetadataStore({ dbPath: ':memory:' }),
        blocks: new HostBlockStore(tmpDir()),
    }),
    capabilities: { symlinks: true, hardLinks: true, permissions: true,
        pwrite: true, fsync: false, copy: true, readDirStat: true, ... },
});

// SQLite + S3 ChunkedVFS (uses MinIO container, see "S3 Testing with MinIO" below)
defineVfsConformanceTests({
    name: 'ChunkedVFS (SQLite + S3)',
    createFs: () => createChunkedVfs({
        metadata: new SqliteMetadataStore({ dbPath: ':memory:' }),
        blocks: new S3BlockStore({
            bucket: minio.bucket,
            prefix: `test-${Date.now()}-${Math.random().toString(36).slice(2, 8)}/`,
            region: 'us-east-1',
            endpoint: minio.endpoint,
            credentials: {
                accessKeyId: minio.accessKeyId,
                secretAccessKey: minio.secretAccessKey,
            },
        }),
    }),
    capabilities: { symlinks: true, hardLinks: true, permissions: true,
        pwrite: true, fsync: false, copy: true, readDirStat: true, ... },
});

// With write buffering enabled
defineVfsConformanceTests({
    name: 'ChunkedVFS (InMemory + InMemory, buffered)',
    createFs: () => createChunkedVfs({
        metadata: new InMemoryMetadataStore(),
        blocks: new InMemoryBlockStore(),
        writeBuffering: true,
    }),
    capabilities: { symlinks: true, hardLinks: true, pwrite: true,
        fsync: true, copy: true, ... },
});

// Pass-through backend
defineVfsConformanceTests({
    name: 'HostDirBackend',
    createFs: () => new HostDirBackend(tmpDir()),
    capabilities: { symlinks: false, hardLinks: false, permissions: true,
        pwrite: true, fsync: false, copy: false, readDirStat: false, ... },
});
```

**Core test cases** (always run):

- writeFile + readFile round-trip (string and binary)
- writeFile + readTextFile round-trip
- writeFile auto-creates parent directories
- writeFile overwrites existing file
- readFile on nonexistent path throws ENOENT
- readFile on directory throws EISDIR
- exists returns true for files, true for directories, false for nonexistent
- stat returns correct size, mode, isDirectory, timestamps
- stat follows symlinks (if symlinks capability)
- lstat does NOT follow symlinks, returns isSymbolicLink: true (if symlinks capability)
- removeFile deletes file
- removeFile on nonexistent path throws ENOENT
- removeFile on directory throws EISDIR
- readDir lists children (excludes . and ..)
- readDirWithTypes returns correct types
- rename file (same directory)
- rename file (cross directory)
- rename overwrites existing destination
- rename directory
- realpath normalizes path

**pwrite test cases** (gated on pwrite capability):

- pwrite at offset 0 (overwrite start of file)
- pwrite at middle of file
- pwrite beyond EOF extends file (gap filled with zeros)
- pwrite spanning chunk boundaries (for chunked backends)
- pwrite + pread round-trip at same offset
- pwrite does not affect bytes outside written range
- Multiple sequential pwrites build up file content
- pwrite to empty file at offset 0

**Concurrency test cases** (always run for chunked backends):

- Two concurrent pwrites to different offsets in same file: both succeed, no data loss
- Two concurrent pwrites to same offset: one wins, no corruption (both produce valid final state)
- Concurrent pwrite + readFile: readFile returns consistent data (not half-written)
- Concurrent writeFile + pread: no crash, returns either old or new data

**fsync test cases** (gated on fsync capability):

- pwrite + fsync + readFile returns written data
- pwrite without fsync: readFile still returns written data (buffered reads see dirty data)
- Multiple pwrites + single fsync: all data visible after fsync
- fsync on nonexistent path: no error (silent no-op)

**copy test cases** (gated on copy capability):

- copy file: content matches original
- copy file: modifying copy does not affect original
- copy file: metadata (mode, size) matches original
- copy preserves chunked storage (no inline demotion for large files)

**readDirStat test cases** (gated on readDirStat capability):

- readDirStat returns same entries as readDir
- Each entry has valid stat fields (size, mode, timestamps)
- Results include directories, files, symlinks with correct types

**Symlink test cases** (gated on symlinks capability):

- symlink + readlink round-trip
- symlink resolution for file access (readFile through symlink)
- lstat on symlink returns isSymbolicLink: true
- stat on symlink returns target's metadata
- Dangling symlink: stat throws ENOENT, lstat succeeds
- Symlink loop (A -> B -> A): throws ELOOP
- Deep symlink chain (41 levels): throws ELOOP
- removeFile on symlink removes link, not target

**Hard link test cases** (gated on hardLinks capability):

- link creates second name for same file
- Writing via one name is visible via the other
- Removing one name: file still accessible via other name
- nlink decremented on removeFile, data deleted only when nlink reaches 0
- link to directory throws EPERM

**Permission test cases** (gated on permissions capability):

- chmod changes mode bits
- chmod preserves file type bits (regular file stays regular)
- chown changes uid/gid

**Truncate test cases** (gated on truncate capability):

- Truncate shorter: file shrinks, content preserved up to new length
- Truncate to 0: file becomes empty
- Truncate longer: file grows, new bytes are zeros
- Truncate at exact inlineThreshold boundary: correct storage mode transition
- Truncate inline file to chunked size: promotes to chunked
- Truncate chunked file to inline size: demotes to inline

**Edge case test cases**:

- Empty file: stat.size == 0, readFile returns empty Uint8Array
- File at exactly inlineThreshold bytes: stored inline
- File at inlineThreshold + 1 bytes: stored chunked
- File at exactly chunkSize bytes: one chunk
- File at chunkSize + 1 bytes: two chunks
- pread with length 0: returns empty Uint8Array
- pread at offset == file size: returns empty Uint8Array
- pread at offset > file size: returns empty Uint8Array
- writeFile with empty content: creates empty file (size 0, inline)
- Very long filename (255 chars): succeeds
- Deeply nested path (20 levels): succeeds

### Layer 2: Block Store Tests

**Location**: `secure-exec-1/packages/core/src/test/block-store-conformance.ts`

Tests the `FsBlockStore` interface in isolation.

```typescript
interface BlockStoreTestConfig {
    name: string;
    createStore: () => Promise<FsBlockStore> | FsBlockStore;
    cleanup?: () => Promise<void>;
    capabilities: {
        copy: boolean;
    };
}

function defineBlockStoreTests(config: BlockStoreTestConfig): void;
```

**Drivers that register:**

```typescript
defineBlockStoreTests({
    name: 'InMemoryBlockStore',
    createStore: () => new InMemoryBlockStore(),
    capabilities: { copy: true },
});

defineBlockStoreTests({
    name: 'HostBlockStore',
    createStore: () => new HostBlockStore(tmpDir()),
    cleanup: () => rimraf(tmpDir()),
    capabilities: { copy: true },
});

defineBlockStoreTests({
    name: 'S3BlockStore',
    createStore: () => new S3BlockStore({
        bucket: minio.bucket,
        prefix: `bs-test-${Date.now()}-${Math.random().toString(36).slice(2, 8)}/`,
        region: 'us-east-1',
        endpoint: minio.endpoint,
        credentials: {
            accessKeyId: minio.accessKeyId,
            secretAccessKey: minio.secretAccessKey,
        },
    }),
    capabilities: { copy: true },
});
```

**Test cases:**

- write + read round-trip (small data)
- write + read round-trip (large data, >4MB)
- readRange: partial read within a block
- readRange: offset at start of block
- readRange: offset at end of block
- readRange: beyond block size returns available bytes (short read)
- read nonexistent key: throws ENOENT
- readRange nonexistent key: throws ENOENT
- delete + read: throws ENOENT
- delete nonexistent key: no error
- deleteMany: deletes all specified keys
- deleteMany with some nonexistent keys: no error, deletes existing ones
- deleteMany with empty array: no-op
- write overwrites existing key
- copy round-trip (if copy capability): dst has same content as src
- copy: modifying dst does not affect src
- copy nonexistent src: throws ENOENT

### Layer 3: Metadata Store Tests

**Location**: `secure-exec-1/packages/core/src/test/metadata-store-conformance.ts`

Tests the `FsMetadataStore` interface in isolation.

```typescript
interface MetadataStoreTestConfig {
    name: string;
    createStore: () => Promise<FsMetadataStore> | FsMetadataStore;
    cleanup?: () => Promise<void>;
    capabilities: {
        versioning: boolean;
    };
}

function defineMetadataStoreTests(config: MetadataStoreTestConfig): void;
```

**Drivers that register:**

```typescript
defineMetadataStoreTests({
    name: 'InMemoryMetadataStore',
    createStore: () => new InMemoryMetadataStore(),
    capabilities: { versioning: false },
});

defineMetadataStoreTests({
    name: 'SqliteMetadataStore',
    createStore: () => new SqliteMetadataStore({ dbPath: ':memory:' }),
    capabilities: { versioning: true },
});
```

**Inode test cases:**

- createInode returns unique ino each time
- getInode returns correct metadata for created inode
- updateInode modifies specified fields only
- deleteInode: getInode returns null afterward
- getInode on never-created ino returns null

**Dentry test cases:**

- createDentry + lookup round-trip
- lookup on nonexistent name returns null
- listDir returns all children
- listDir on empty directory returns empty array
- listDirWithStats returns entries with full inode metadata
- removeDentry: lookup returns null afterward
- removeDentry does NOT delete the child inode
- createDentry with duplicate name throws EEXIST
- renameDentry within same parent
- renameDentry across different parents
- renameDentry to existing name: overwrites destination

**Path resolution test cases:**

- resolvePath("/") returns root inode (1)
- resolvePath single component ("/foo")
- resolvePath multi-component ("/foo/bar/baz")
- resolvePath follows symlinks
- resolvePath throws ENOENT for missing intermediate component
- resolvePath throws ENOENT for missing final component
- resolvePath throws ELOOP for circular symlinks (A -> B -> A)
- resolvePath throws ELOOP at depth 41
- resolveParentPath returns parent inode and final name
- resolveParentPath does NOT follow symlink in final component
- resolveParentPath throws ENOENT for missing intermediate component

**Chunk mapping test cases:**

- setChunkKey + getChunkKey round-trip
- getChunkKey for missing chunk returns null
- getAllChunkKeys returns ordered list
- getAllChunkKeys on file with no chunks returns empty array
- setChunkKey overwrites existing mapping
- deleteAllChunks removes all mappings, returns deleted keys
- deleteChunksFrom removes chunks >= startIndex, returns deleted keys
- deleteChunksFrom with startIndex beyond last chunk: no-op, returns empty

**Transaction test cases:**

- transaction commits on success
- transaction rolls back on thrown error
- Nested transactions (if supported): inner commit visible to outer
- Concurrent transactions (separate async contexts): both succeed if non-conflicting

**Symlink test cases:**

- createInode with symlinkTarget + readSymlink round-trip
- readSymlink on non-symlink inode: throws

**Versioning test cases** (gated on versioning capability):

- createVersion returns incrementing version numbers
- listVersions returns all versions newest first
- getVersion returns correct metadata
- getVersionChunkMap returns chunk keys at snapshot time
- restoreVersion reverts current chunk map
- After restore, pread returns old version's data
- deleteVersions removes specified versions
- deleteVersions returns orphaned block keys (not referenced by any remaining version or current state)
- createVersion + write new data + listVersions: old version has old size, new version not created yet (explicit snapshots only)

### S3 Testing with MinIO

S3BlockStore tests and ChunkedVFS(SQLite + S3) conformance tests run against a local MinIO container. The existing Docker test infrastructure in `agent-os/packages/core/src/test/docker.ts` provides `startMinioContainer()` which:

1. Pulls `minio/minio:latest` and starts the container with a random host port
2. Waits for health check (`mc ready local`)
3. Creates a test bucket via `mc mb`
4. Returns a `MinioContainerHandle` with `endpoint`, `accessKeyId`, `secretAccessKey`, `bucket`

Test file setup pattern for S3 tests:

```typescript
import { beforeAll, afterAll } from "vitest";
import type { MinioContainerHandle } from "@rivet-dev/agent-os/test/docker";
import { startMinioContainer } from "@rivet-dev/agent-os/test/docker";

let minio: MinioContainerHandle;

beforeAll(async () => {
    minio = await startMinioContainer({ healthTimeout: 60_000 });
}, 90_000);

afterAll(async () => {
    if (minio) await minio.stop();
});

// Each test/suite uses a unique key prefix to avoid cross-test interference:
const prefix = `test-${Date.now()}-${Math.random().toString(36).slice(2, 8)}/`;

// Then pass to S3BlockStore:
new S3BlockStore({
    bucket: minio.bucket,
    prefix,
    region: "us-east-1",
    endpoint: minio.endpoint,
    credentials: {
        accessKeyId: minio.accessKeyId,
        secretAccessKey: minio.secretAccessKey,
    },
});
```

Requirements:
- Docker must be installed and running on the host
- Tests that need MinIO should have generous timeouts (90s for setup, 60s per test)
- Each test run uses a unique prefix so parallel test runs don't collide
- The MinIO container is ephemeral (`--rm`). All data is lost on stop.
- S3BlockStore must use `forcePathStyle: true` for MinIO compatibility (endpoint-based addressing instead of virtual-hosted-style)

### Layer 4: ChunkedVFS Integration Tests

**Location**: `secure-exec-1/packages/core/tests/chunked-vfs.test.ts`

Tests ChunkedVFS internals not visible through the VFS interface.

**Tiered storage test cases:**

- Small file stays inline (spy on block store: no write calls)
- File crossing inlineThreshold promotes to chunked (block store has chunks)
- pwrite that pushes inline file past threshold: promotes correctly
- Truncate below threshold demotes chunked to inline
- Truncate above threshold promotes inline to chunked
- writeFile with content exactly at threshold: stored inline
- writeFile with content at threshold + 1: stored chunked

**Chunk math test cases:**

- pwrite to middle of file touches only one chunk
- pwrite spanning two chunks modifies both
- pwrite spanning three chunks modifies all three
- writeFile on large file creates correct number of chunks (size / chunkSize, rounded up)
- readFile on chunked file concatenates all chunks correctly
- Sparse file: pwrite at high offset, read intervening range returns zeros
- Last chunk may be smaller than chunkSize

**Concurrency test cases:**

- Two concurrent pwrites to same inode are serialized (per-inode mutex)
- pwrite during ongoing pwrite: second waits for first to complete
- Inline-to-chunked promotion under concurrent writes: no double promotion

**Write buffering test cases** (with writeBuffering enabled):

- pwrite buffers data (spy on block store: no immediate write)
- pread sees buffered data before fsync
- readFile sees buffered data before fsync
- stat.size reflects buffered writes
- fsync flushes dirty chunks to block store
- After fsync, block store has written data
- Multiple pwrites to same chunk coalesce (single block store write on fsync)
- Auto-flush fires after configured interval
- fsync on stale path (file renamed): no error

**Versioning test cases** (with versioning enabled):

- Each pwrite creates a new block key (old key preserved)
- createVersion snapshots current chunk map
- After write, old version's block keys still exist
- restoreVersion: pread returns old data
- pruneVersions with count policy: keeps N newest, deletes rest
- pruneVersions with age policy: keeps recent, deletes old
- pruneVersions with deferred policy: deletes metadata only, not blocks
- pruneVersions returns orphaned keys for immediate deletion
- collectGarbage finds and deletes unreferenced blocks

---

## Implementation Order

1. **Add pwrite to VirtualFileSystem interface** and update MountTable to delegate it. Update kernel `fdPwrite` to call `vfs.pwrite()` instead of read-modify-write. This is the foundation that makes chunked storage useful.

2. **FsMetadataStore interface + InMemoryMetadataStore**. Pure JS Map-based metadata store with transaction support, path resolution, chunk mapping.

3. **FsBlockStore interface + InMemoryBlockStore**. Trivial Map-based implementation.

4. **ChunkedVFS** composing them into a VirtualFileSystem. Tiered storage (inline/chunked), chunk math, per-inode mutex, all data flows except versioning and write buffering.

5. **VFS conformance test suite** in secure-exec. Port and extend existing `defineFsDriverTests` from agent-os. Add pwrite, concurrency, and edge case tests. Run against ChunkedVFS(InMemory + InMemory).

6. **Block store conformance tests + metadata store conformance tests**. Ensure each layer is tested independently.

7. **SqliteMetadataStore** implementation. Register with metadata store conformance tests.

8. **HostBlockStore** implementation (blocks as files on host FS). Register with block store conformance tests. Register ChunkedVFS(SQLite + HostBlockStore) with VFS conformance tests.

9. **S3BlockStore** implementation (in agent-os). Register with block store conformance tests. Register ChunkedVFS(SQLite + S3) with VFS conformance tests.

10. **Add fsync to VFS interface.** Implement write buffering in ChunkedVFS. Update kernel to call fsync on last FD close. Run conformance tests with buffered variant.

11. **Add copy and readDirStat** optimizations to VFS interface and ChunkedVFS.

12. **Versioning** (optional). Add versioning metadata to SqliteMetadataStore. Add versioning API to ChunkedVFS. Add retention policies.

13. **Kernel cleanup.** Remove InMemoryFileSystem fast paths. Remove old inode-table.ts. Update kernel to work purely through VFS interface.

14. **Delete old backends.** Remove agent-os fs-sqlite, fs-postgres packages. Rewrite agent-os fs-s3 as thin S3BlockStore.

---

## Open Questions

- **Chunk size**: 4 MB default. Should this be configurable per-mount? Smaller chunks (1 MB) reduce write amplification for partial writes but increase metadata overhead and S3 request count.
- **Unlinked-file-read**: Full POSIX unlink-while-open requires inode-based I/O in the VFS. Deferred to a future version. For v1, unlinking a file with open FDs may cause subsequent reads to fail.
- **Sparse file hole-punching**: The chunk map naturally supports sparse files. Do we need explicit `fallocate(FALLOC_FL_PUNCH_HOLE)` to free chunks in the middle of a file?
- **Large directory performance**: listDir on directories with 10k+ entries may be slow for SQLite. Consider pagination or streaming.

---

## Documentation

When implementing this spec, update documentation in secure-exec:

- **`~/secure-exec-1/CLAUDE.md`**: Update the Virtual Filesystem Design Reference section to reflect the new architecture (ChunkedVFS, FsMetadataStore/FsBlockStore separation, tiered storage, etc.). Remove references to the old monolithic VFS backends.
- **`~/secure-exec-1/CLAUDE.md`**: Document the new exported test suites (VFS conformance, block store conformance, metadata store conformance) so that external packages know how to register their implementations.
- **`~/secure-exec-1/CLAUDE.md`**: Document the kernel changes (pwrite delegation, fsync on last FD close, removal of InMemoryFileSystem fast paths).
- **`~/r16/agent-os/CLAUDE.md`**: Update the Virtual Filesystem Design Reference section to match. Remove references to deleted packages (fs-sqlite, fs-postgres). Document that S3BlockStore is the only remaining agent-os FS package.
- **Inline JSDoc**: All exported interfaces (`VirtualFileSystem`, `FsMetadataStore`, `FsBlockStore`, `ChunkedVfsOptions`) must have JSDoc comments describing the contract, error behavior, and usage patterns.
- **Test suite docs**: Each conformance test suite file (`vfs-conformance.ts`, `block-store-conformance.ts`, `metadata-store-conformance.ts`) should have a file-level comment explaining how external implementations register and what capabilities flags control.
