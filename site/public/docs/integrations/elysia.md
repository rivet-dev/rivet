# Elysia

Integrate Rivet with Elysia for fast TypeScript web applications

Elysia is a fast and type-safe web framework for Bun. Rivet integrates seamlessly with Elysia using the `.mount()` method.

	Check out the complete example

## Installation

Install Elysia alongside Rivet:

```bash
npm install elysia
# or with bun
bun add elysia
```

## Basic Setup

Set up your Rivet Actors:

```typescript
// registry.ts
const counter = actor(,
  actions: ,
    getCount: (c) => c.state.count,
  },
});

const registry = setup(,
});
```

Mount Rivet into your Elysia application:

```typescript
// server.ts
const  = registry.createServer();

// Setup Elysia app
const app = new Elysia()
  // Mount Rivet handler
  .mount("/registry", handler)
  // Add your API routes
  .post("/increment/:name", async () => `;
  })
  .get("/count/:name", async () => ;
  })
  .listen(8080);

console.log("Server running at http://localhost:8080");
```