# Node.js & Bun Quickstart

Get started with Rivet Actors in Node.js and Bun

```sh
npm install @rivetkit/actor
```

Create a simple counter actor:

```ts }
const counter = actor(,
	actions: ,
		getCount: (c) => c.state.count,
	},
});

const registry = setup(,
});
```

Choose your preferred web framework:

```ts }
// Start Rivet with file system driver (for development)
const  = registry.createServer();

// Setup Hono app
const app = new Hono();

// Example API endpoint
app.post("/increment/:name", async (c) => );
});

// Start server with Rivet
serve(app);
```

```ts }
// Start Rivet
const  = registry.createServer();

// Setup Express app
const app = express();
app.use(express.json());

// Mount Rivet handler
app.use("/registry", handler);

// Example API endpoints
app.post("/increment/:name", async (req, res) =>  = req.params;

      const counter = client.counter.getOrCreate(name);
      const newCount = await counter.increment(1);
      
      res.json();
});

app.listen(8080, () => );
```

```ts }
// Start Rivet
const  = registry.createServer();

// Setup Elysia app
const app = new Elysia()
	.mount("/registry", handler)
	.post("/increment/:name", async () =>  = params;

		const counter = client.counter.getOrCreate(name);
		const newCount = await counter.increment(1);

		return ;
	})
	.listen(8080);

console.log("Server running at http://localhost:8080");
```

The `/registry` endpoint is automatically mounted by Rivet and is required for client communication. When using `serve()` with Hono, this is handled automatically.

```sh }
npx tsx --watch server.ts
```

```sh }
bun --watch server.ts
```

Your server is now running at `http://localhost:8080`

Test your counter actor using HTTP requests:

```ts }
// Increment counter
const response = await fetch("http://localhost:8080/increment/my-counter", );

const result = await response.json();
console.log("Count:", result.count); // 1
```

```sh curl
# Increment counter
curl -X POST http://localhost:8080/increment/my-counter
```

## Configuration Options

### Connect Frontend To The Rivet Actor

Create a type-safe client to connect from your frontend:

```ts }
// Create typed client
const client = createClient("http://localhost:8080");

// Use the counter actor directly
const counter = client.counter.getOrCreate(["my-counter"]);

// Call actions
const count = await counter.increment(3);
console.log("New count:", count);

// Get current state
const currentCount = await counter.getCount();
console.log("Current count:", currentCount);

// Listen to real-time events
const connection = counter.connect();
connection.on("countChanged", (newCount) => );

// Increment through connection
await connection.increment(1);
```

See the [JavaScript client documentation](/clients/javascript) for more information.

```tsx }
const client = createClient("http://localhost:8080");
const  = createRivetKit(client);

function Counter() );

	counter.useEvent("countChanged", (newCount: number) => );

	const increment = async () => ;

	return (
		
			Count: 
			Increment
		
	);
}
```

See the [React documentation](/clients/react) for more information.

```rust }
use rivetkit_client::;
use serde_json::json;

#[tokio::main]
async fn main() -> Result> ", count);
    }).await;
    
    // Call increment action
    let result = counter.action("increment", vec![json!(1)]).await?;
    println!("New count: ", result);
    
    Ok(())
}
```

See the [Rust client documentation](/clients/rust) for more information.