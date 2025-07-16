# Hono

Integrate Rivet with Hono for ultra-fast web applications

Hono is an ultra-fast web framework that works on any runtime. Rivet integrates seamlessly with Hono through the `serve()` method.

	Check out the complete example

## Installation

Install Hono alongside Rivet:

```bash
npm install hono
```

## Basic Setup

Set up your Rivet Actor:

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

Use Rivet's `serve()` method with your Hono app:

```typescript
// server.ts
// Start Rivet
const  = registry.createServer();

// Setup Hono app
const app = new Hono();

// Add your API routes
app.post("/increment/:name", async (c) => ));
  const amount = body.amount || 1;
  
  try );
  } catch (error) , 500);
  }
});

app.get("/count/:name", async (c) => );
  } catch (error) , 500);
  }
});

// Start server with Rivet integration
serve(app);
```