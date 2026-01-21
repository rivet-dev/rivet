# File System

> Source: `src/content/docs/self-hosting/filesystem.mdx`
> Canonical URL: https://rivet.gg/docs/self-hosting/filesystem
> Description: The file system backend stores all data on the local disk. This is suitable for single-node deployments, development, and testing.

---
The file system backend does not support multi-node deployments. Use [PostgreSQL](/docs/self-hosting/postgres) for production.

## Configuration

```json {{"title":"Configuration file"}}
{
  "database": {
    "file_system": {
      "path": "/var/lib/rivet/data"
    }
  }
}
```

```bash {{"title":"Environment variables"}}
RIVET__database__file_system__path="/var/lib/rivet/data"
```

## Default Paths

If no path is specified, Rivet uses platform-specific default locations:

- Linux: `~/.local/share/rivet-engine/db`
- macOS: `~/Library/Application Support/rivet-engine/db`
- Windows: `%APPDATA%\rivet-engine\db`

When running in a container or as a service, the path defaults to `./data/db` relative to the working directory.

## When to Use File System

The file system backend is ideal for:

- Local development
- Single-node deployments
- Testing and prototyping
- Air-gapped environments without database infrastructure

For production deployments with multiple nodes or high availability requirements, use [PostgreSQL](/docs/self-hosting/postgres) instead.

_Source doc path: /docs/self-hosting/filesystem_
