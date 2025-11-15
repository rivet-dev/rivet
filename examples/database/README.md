# Database Notes for Rivet

Example project demonstrating persistent data storage and real-time updates with [Rivet](https://www.rivet.dev/).

[Learn More →](https://github.com/rivet-dev/rivet)

[Discord](https://rivet.dev/discord) — [Documentation](https://www.rivet.dev/) — [Issues](https://github.com/rivet-dev/rivet/issues)

## Getting Started

### Prerequisites

- Node.js 18+

### Installation

```sh
git clone https://github.com/rivet-dev/rivet
cd rivet/examples/database
npm install
```

### Development

```sh
npm run dev
```

Open your browser to `http://localhost:3000`

## Features

- **Persistent Storage**: Notes are automatically saved and persist across sessions
- **Real-time Updates**: Changes are instantly synchronized across all connected clients
- **User Authentication**: Demonstrates basic authentication with token validation
- **Multi-user Support**: Switch between different users to see isolated data
- **CRUD Operations**: Create, read, update, and delete notes
- **Edit in Place**: Click edit to modify notes inline
- **Auto-sorting**: Notes are automatically sorted by last modified date

## How it works

This example demonstrates:

1. **Actor State Management**: Using Rivet actors to manage persistent application state
2. **Authentication**: Basic token-based authentication for user identification
3. **Real-time Events**: Broadcasting changes to all connected clients using actor events
4. **State Persistence**: Actor state is automatically persisted between sessions
5. **Connection State**: Handle connection status and graceful degradation

## Architecture

- **Backend**: Rivet actor that manages note storage and user authentication
- **Frontend**: React application with real-time updates via Rivet hooks
- **State Management**: Each user gets their own actor instance for data isolation
- **Authentication**: Mock token-based auth (replace with real auth in production)

## Usage

1. Start the development server
2. Select a user from the dropdown to see their notes
3. Add new notes using the input field
4. Edit notes by clicking the "Edit" button
5. Delete notes with the "Delete" button
6. Open multiple tabs or users to see real-time synchronization

## Extending

This example can be extended with:

- Real database integration (PostgreSQL, MongoDB, etc.)
- Proper JWT authentication
- User registration and management
- Note sharing and collaboration
- Rich text editing
- File attachments
- Search and filtering

## License

Apache 2.0