# tRPC

Integrate Rivet with tRPC for end-to-end type-safe APIs

tRPC provides end-to-end type safety for your APIs. Rivet integrates seamlessly with tRPC, allowing you to create type-safe procedures that call Rivet Actors.

	Check out the complete example

## Installation

Install tRPC alongside Rivet:

```bash
npm install @trpc/server @trpc/client zod
npm install -D @trpc/next # if using Next.js
```

## Basic Setup

Set up your Rivet Actors:

```typescript
// registry.ts
const counter = actor(,
  actions: ,
    getCount: (c) => c.state.count,
    reset: (c) => ,
  },
});

const registry = setup(,
});
```

Create your tRPC router that uses Rivet:

```typescript
// server.ts
// Start Rivet
const  = registry.createServer();

// Initialize tRPC
const t = initTRPC.create();

// Create tRPC router with Rivet integration
const appRouter = t.router())
      .mutation(async () => ;
      }),
    
    get: t.procedure
      .input(z.object())
      .query(async () => ;
      }),
    
    reset: t.procedure
      .input(z.object())
      .mutation(async () => ;
      }),
  }),
});

// Export type for client
type AppRouter = typeof appRouter;

// Create HTTP server
const server = createHTTPServer();

server.listen(3001);
console.log("tRPC server listening at http://localhost:3001");
```

Create a type-safe tRPC client:

```typescript
// client.ts
const trpc = createTRPCProxyClient(),
  ],
});

// Usage examples
async function examples() );
  console.log(result); // 
  
  // Get counter value
  const value = await trpc.counter.get.query();
  console.log(value); // 
  
  // Reset counter
  const reset = await trpc.counter.reset.mutate();
  console.log(reset); // 
}
```