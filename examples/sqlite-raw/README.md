# SQLite Raw Example

This example demonstrates using the raw SQLite driver with RivetKit actors.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/sqlite-raw
npm install
npm run dev
```


## Features

- **Raw SQLite API**: Direct SQL access using `@rivetkit/db/raw`
- **Migrations on Wake**: Uses `onMigrate` to create tables on actor wake
- **Todo List**: Simple CRUD operations with raw SQL queries

## Running the Example

```bash
pnpm install
pnpm dev
```

## Usage

The example creates a `todoList` actor with the following actions:

- `addTodo(title: string)` - Add a new todo
- `getTodos()` - Get all todos
- `toggleTodo(id: number)` - Toggle todo completion status
- `deleteTodo(id: number)` - Delete a todo

## Code Structure

- `src/registry.ts` - Actor definition with database configuration
- `src/server.ts` - Server entry point

## Database

The database uses the KV-backed SQLite VFS, which stores data in a key-value store. The schema is created using raw SQL in the `onMigrate` hook:

```typescript
db: db({
  onMigrate: async (db) => {
    await db.execute(`
      CREATE TABLE IF NOT EXISTS todos (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        title TEXT NOT NULL,
        completed INTEGER DEFAULT 0,
        created_at INTEGER NOT NULL
      )
    `);
  },
})
```

## License

MIT
