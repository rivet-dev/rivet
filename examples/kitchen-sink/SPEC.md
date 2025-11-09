# Kitchen Sink Example Specification

## Overview
Comprehensive example demonstrating all RivetKit features with a simple React frontend.

## UI Structure

### Header (Always visible)

**Configuration Row 1:**
- **Transport**: WebSocket ↔ SSE toggle
- **Encoding**: JSON ↔ CBOR ↔ Bare dropdown
- **Connection Mode**: Handle ↔ Connection toggle

**Actor Management Row 2:**
- **Actor Name**: dropdown
- **Key**: input
- **Region**: input
- **Input JSON**: textarea
- **Buttons**: `create()` | `get()` | `getOrCreate()` | `getForId()` (when no actor connected)
- **OR Button**: `dispose()` (when actor connected)

**Status Row 3:**
- Connection status indicator with actor info (Name | Key | Actor ID | Status)

### Main Content (8 Tabs - disabled until actor connected)

#### 1. Actions
- Action name dropdown/input
- Arguments JSON textarea
- Call button
- Response display

#### 2. Events
- Event listener controls
- Live event feed with timestamps
- Event type filter

#### 3. Schedule
- `schedule.at()` with timestamp input
- `schedule.after()` with delay input
- Scheduled task history
- Custom alarm data input

#### 4. Sleep
- `sleep()` button
- Sleep timeout configuration
- Lifecycle event display (`onWake`, `onSleep`)

#### 5. Connections (Only visible in Connection Mode)
- Connection state display
- Connection info (ID, transport, encoding)
- Connection-specific event history

#### 6. Raw HTTP
- HTTP method dropdown
- Path input
- Headers textarea
- Body textarea
- Send button & response display

#### 7. Raw WebSocket
- Message input (text/binary toggle)
- Send button
- Message history (sent/received with timestamps)

#### 8. Metadata
- Actor name, tags, region display

## User Flow
1. Configure transport/encoding/connection mode
2. Fill actor details (name, key, region, input)
3. Click create/get/getOrCreate/getForId
4. Status shows connection info, tabs become enabled
5. Use tabs to test features
6. Click dispose() to disconnect and return to step 1

## Actors

### 1. `demo` - Main comprehensive actor
- All action types (sync/async/promise)
- Events and state management
- Scheduling capabilities (`schedule.at()`, `schedule.after()`)
- Sleep functionality with configurable timeout
- Connection state support
- Lifecycle hooks (`onWake`, `onSleep`, `onConnect`, `onDisconnect`)
- Metadata access

### 2. `http` - Raw HTTP handling
- `onFetch()` handler
- Multiple endpoint examples
- Various HTTP methods support

### 3. `websocket` - Raw WebSocket handling
- `onWebSocket()` handler
- Text and binary message support
- Connection lifecycle management

## Features Demonstrated

### Core Features
- **Actions**: sync, async, promise-based with various input types
- **Events**: broadcasting to connected clients
- **State Management**: actor state + per-connection state
- **Scheduling**: `schedule.at()`, `schedule.after()` with alarm handlers
- **Force Sleep**: `sleep()` method with configurable sleep timeout
- **Lifecycle Hooks**: `onWake`, `onSleep`, `onConnect`, `onDisconnect`

### Configuration Options
- **Transport**: WebSocket vs Server-Sent Events
- **Encoding**: JSON, CBOR, Bare

### Raw Protocols
- **Raw HTTP**: Direct HTTP request handling
- **Raw WebSocket**: Direct WebSocket connection handling

### Connection Patterns
- **Handle Mode**: Fire-and-forget action calls
- **Connection Mode**: Persistent connection with real-time events

### Actor Management
- **Create**: `client.actor.create(key, opts)`
- **Get**: `client.actor.get(key, opts)`
- **Get or Create**: `client.actor.getOrCreate(key, opts)`
- **Get by ID**: `client.actor.getForId(actorId, opts)`
- **Dispose**: `client.dispose()` - disconnect all connections
