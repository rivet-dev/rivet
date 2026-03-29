> **Preview** This package is in preview and may have breaking changes.

# @rivet-dev/agent-os-google-drive

Google Drive-backed `FsBlockStore` for agentOS. Stores file content blocks as
Google Drive files inside a configurable folder, enabling persistent cloud
storage via the Google Drive API v3.

## Usage

```ts
import { GoogleDriveBlockStore } from "@rivet-dev/agent-os-google-drive";
import { createChunkedVfs, SqliteMetadataStore } from "@secure-exec/core";

const blocks = new GoogleDriveBlockStore({
  credentials: {
    clientEmail: "...",
    privateKey: "...",
  },
  folderId: "your-google-drive-folder-id",
});

const vfs = createChunkedVfs({
  metadata: new SqliteMetadataStore({ dbPath: ":memory:" }),
  blocks,
});
```

## Configuration

| Option | Type | Required | Description |
|--------|------|----------|-------------|
| `credentials` | `{ clientEmail: string; privateKey: string }` | Yes | Google service account credentials |
| `folderId` | `string` | Yes | Google Drive folder ID where blocks are stored |
| `keyPrefix` | `string` | No | Optional prefix for block file names |

## Rate Limits

Google Drive API has a rate limit of approximately 10 queries/sec/user. Heavy
I/O workloads may experience throttling. Consider using write buffering in
ChunkedVFS (`writeBuffering: true`) to reduce API calls.
