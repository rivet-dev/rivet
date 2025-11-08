# Sync Contacts for Rivet

Example project demonstrating offline-first contact synchronization with conflict resolution using [Rivet](https://www.rivet.dev/).

[Learn More →](https://github.com/rivet-dev/rivet)

[Discord](https://rivet.dev/discord) — [Documentation](https://www.rivet.dev/) — [Issues](https://github.com/rivet-dev/rivet/issues)

## Getting Started

### Prerequisites

- Node.js 18+

### Installation

```sh
git clone https://github.com/rivet-dev/rivet
cd rivet/examples/sync
npm install
```

### Development

```sh
npm run dev
```

Open your browser to `http://localhost:3000`

## Features

- **Offline-First Sync**: Add contacts locally, sync when connection available
- **Conflict Resolution**: "Last write wins" conflict resolution using timestamps
- **Real-time Updates**: See changes from other clients immediately
- **Soft Deletes**: Deleted contacts are marked as deleted, not removed
- **Periodic Sync**: Automatic background synchronization every 5 seconds
- **Manual Sync**: Force synchronization with "Sync Now" button
- **Sync Statistics**: Track total contacts, deletions, and last sync time
- **Connection Status**: Visual indicators for sync status (Synced/Syncing/Offline)

## How it works

This contact sync system demonstrates:

1. **Offline-First Architecture**: Changes are applied locally first for immediate UI feedback
2. **Conflict Resolution**: Server uses timestamp-based "last write wins" strategy
3. **Event Broadcasting**: Real-time updates sent to all connected clients
4. **Soft Delete Pattern**: Deleted contacts marked with empty name instead of removal
5. **Periodic Synchronization**: Background sync every 5 seconds to catch remote changes
6. **Optimistic Updates**: UI updates immediately before server confirmation

## Architecture

- **Backend**: Rivet actor managing contact state and synchronization logic
- **Frontend**: React application with offline-first contact management
- **Sync Strategy**: Timestamp-based conflict resolution with periodic reconciliation
- **State Management**: Server-side persistence with client-side optimistic updates

## Synchronization Flow

### Adding Contacts
```typescript
// 1. Add locally for immediate UI feedback
setContacts(prev => [...prev, newContact]);

// 2. Push to server for persistence and broadcast
await actor.pushChanges([newContact]);
```

### Conflict Resolution
```typescript
// Server-side: Last write wins based on timestamp
if (!existing || existing.updatedAt < contact.updatedAt) {
    state.contacts[contact.id] = contact;
}
```

### Periodic Sync
```typescript
// Every 5 seconds:
// 1. Get remote changes since last sync
const changes = await actor.getChanges(lastSyncTime);

// 2. Apply remote changes locally
// 3. Push any local changes to server
// 4. Update last sync timestamp
```

## Sync Strategies

This example implements **Last Write Wins** conflict resolution, but the pattern supports other strategies:

### Last Write Wins (Current Implementation)
- Simple timestamp comparison
- Most recent change takes precedence
- Easy to implement and understand
- Risk of data loss in concurrent edits

### Alternative Strategies
- **Operational Transform**: Transform operations to maintain intent
- **CRDTs**: Conflict-free replicated data types for automatic resolution
- **Three-Way Merge**: Compare base, local, and remote versions
- **User-Prompted Resolution**: Ask user to resolve conflicts manually

## Use Cases

This sync pattern is perfect for:

- **Contact Management**: Personal and business contact lists
- **Note Taking**: Distributed note-taking applications
- **Todo Lists**: Task management with offline support
- **Settings Sync**: User preferences across devices
- **Shopping Lists**: Collaborative shopping with family/friends
- **Inventory Management**: Small business inventory tracking

## Extending

This sync system can be enhanced with:

- **User Authentication**: Per-user contact isolation
- **Categories/Tags**: Organize contacts into groups
- **Import/Export**: Bulk contact operations
- **Search/Filtering**: Find contacts quickly
- **Merge Conflicts**: UI for manual conflict resolution
- **Backup/Restore**: Data protection features
- **Sharing**: Share contacts between users
- **Versioning**: Track contact change history
- **Advanced Sync**: Delta sync for large datasets

## Offline Behavior

### When Offline
- Contacts can still be added/deleted locally
- Changes are queued for next sync
- UI shows "Offline" status
- All functionality remains available

### When Reconnecting
- Automatic sync of queued changes
- Conflict resolution applied
- Status updates to "Syncing" then "Synced"
- Real-time updates resume

## Testing Offline Sync

To test offline functionality:

1. **Add contacts** while online
2. **Disconnect network** (disable WiFi or ethernet)
3. **Add more contacts** - they appear locally
4. **Reconnect network** - contacts sync automatically
5. **Open multiple tabs** - see real-time sync between clients

## Performance Considerations

### Optimization Strategies
- **Delta Sync**: Only sync changes since last sync
- **Batching**: Group multiple changes into single requests
- **Compression**: Compress sync payloads for large datasets
- **Indexing**: Index by timestamp for efficient change queries
- **Pagination**: Handle large contact lists efficiently

### Scalability Notes
- Current implementation stores all contacts in memory
- For production, consider database persistence
- Implement pagination for large contact lists
- Add rate limiting for sync operations
- Consider WebSocket connections for real-time updates

## Error Handling

The system handles various error scenarios:

- **Network Failures**: Fall back to offline mode
- **Server Errors**: Retry with exponential backoff
- **Sync Conflicts**: Automatic resolution with timestamps
- **Invalid Data**: Validation before persistence
- **Connection Loss**: Queue changes for later sync

## License

Apache 2.0