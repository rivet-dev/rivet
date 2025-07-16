# Express

Integrate Rivet with Express.js for Node.js web applications

Express.js is a popular Node.js web framework. Rivet integrates seamlessly with Express using middleware mounting.

	Check out the complete example

## Installation

Install Express alongside Rivet:

```bash
npm install express
npm install -D @types/express
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

Mount Rivet into your Express application:

```typescript
// server.ts
// Start Rivet
const  = registry.createServer();

// Setup Express app
const app = express();

// Enable JSON parsing
app.use(express.json());

// Mount Rivet handler
app.use("/registry", handler);

// Add your API routes
app.post("/increment/:name", async (req, res) =>  = req.body;
  
  try );
  } catch (error) );
  }
});

app.get("/count/:name", async (req, res) => );
  } catch (error) );
  }
});

app.listen(8080, () => );
```