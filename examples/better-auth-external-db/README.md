# Better Auth with External Database for RivetKit

Example project demonstrating authentication integration with [RivetKit](https://rivetkit.org) using Better Auth and SQLite database.

[Learn More →](https://github.com/rivet-dev/rivetkit)

[Discord](https://rivet.dev/discord) — [Documentation](https://rivetkit.org) — [Issues](https://github.com/rivet-dev/rivetkit/issues)

## Getting Started

### Prerequisites

- Node.js 18+
- npm or pnpm

### Installation

```sh
git clone https://github.com/rivet-dev/rivetkit
cd rivetkit/examples/better-auth-external-db
npm install
```

### Development

```sh
npm run dev
```

The database migrations will run automatically on startup. Open your browser to `http://localhost:5173` to see the frontend and the backend will be running on `http://localhost:6420`.

## Features

- **Authentication**: Email/password authentication using Better Auth
- **Protected Actors**: Rivet Actors with authentication via `onAuth` hook
- **Real-time Chat**: Authenticated chat room with real-time messaging
- **External Database**: Shows how to configure Better Auth with external database (SQLite example)

## How It Works

1. **Better Auth Setup**: Configured with SQLite database for persistent user storage (auto-migrated in development)
2. **Protected Actor**: The `chatRoom` actor uses the `onAuth` hook to verify user sessions
3. **Frontend Integration**: React components handle authentication flow and chat interface
4. **Session Management**: Better Auth handles session creation, validation, and cleanup
5. **Auto-Migration**: Database schema is automatically migrated when starting the development server

## Database Commands

- `npm run db:generate` - Generate migration files for database schema changes
- `npm run db:migrate` - Apply migrations to the database (used in production)

## Key Files

- `src/backend/auth.ts` - Better Auth configuration with SQLite database
- `src/backend/registry.ts` - Rivet Actor with authentication
- `src/frontend/components/AuthForm.tsx` - Login/signup form
- `src/frontend/components/ChatRoom.tsx` - Authenticated chat interface
- `auth.sqlite` - SQLite database file (auto-created)

## License

Apache 2.0
