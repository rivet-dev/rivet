# CRDT Collaborative Editor for RivetKit

Example project demonstrating real-time collaborative editing using Conflict-free Replicated Data Types (CRDTs) with [RivetKit](https://rivetkit.org).

[Learn More →](https://github.com/rivet-dev/rivetkit)

[Discord](https://rivet.dev/discord) — [Documentation](https://rivetkit.org) — [Issues](https://github.com/rivet-dev/rivetkit/issues)

## Getting Started

### Prerequisites

- Node.js 18+

### Installation

```sh
git clone https://github.com/rivet-dev/rivetkit
cd rivetkit/examples/crdt
npm install
```

### Development

```sh
npm run dev
```

Open your browser to `http://localhost:3000`

## Features

- **Real-time Collaborative Editing**: Multiple users can edit the same document simultaneously
- **Conflict Resolution**: Uses Yjs CRDTs to automatically resolve editing conflicts
- **Persistent State**: Document changes are automatically persisted
- **Multiple Documents**: Switch between different collaborative documents
- **Live Connection Status**: See when you're connected to the collaboration server

## How it works

This example demonstrates how to build a collaborative editor using:

1. **Yjs**: A high-performance CRDT implementation for building collaborative applications
2. **RivetKit Actors**: Manage document state and synchronize changes between clients
3. **Real-time Updates**: Use RivetKit's event system for instant synchronization
4. **Conflict-free Merging**: Yjs automatically handles concurrent edits without conflicts

## Usage

1. Start the development server
2. Open multiple browser tabs to `http://localhost:3000`
3. Start typing in any tab - changes will appear in real-time across all tabs
4. Try editing the same text simultaneously to see conflict resolution in action
5. Switch between different documents using the document ID field

## Architecture

- **Backend**: RivetKit actor that manages Yjs document state and broadcasts updates
- **Frontend**: React application with Yjs integration for local document management
- **Synchronization**: Binary diffs are sent between clients for efficient updates

## License

Apache 2.0