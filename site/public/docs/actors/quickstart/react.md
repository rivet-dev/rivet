# React Quickstart

Build real-time React applications with Rivet Actors

```sh
npm install @rivetkit/actor @rivetkit/react
```

Create your actor registry on the backend:

```ts backend/registry.ts
const counter = actor(,
	actions: ,
		getCount: (c) => c.state.count,
	},
});

const registry = setup(,
});
```

Start a server to run your actors:

```ts backend/server.ts
registry.runServer();
```

Set up your React application:

```tsx frontend/App.tsx
// Create typed client
const client = createClient("http://localhost:8080");
const  = createRivetKit(client);

function App() );

	// Listen for real-time count updates
	counter.useEvent("countChanged", (newCount: number) => );

	const increment = async () => ;

	const incrementBy = async (amount: number) => ;

	return (
		
			Rivet Counter
			Count: 

					Counter Name:
					 setCounterName(e.target.value)}
						style=}
					/>

					+1
				
				 incrementBy(5)}>
					+5
				
				 incrementBy(10)}>
					+10

				Connection Status: 
				Try opening multiple tabs to see real-time sync.

	);
}

default App;
```

Configure Vite for development:

```ts vite.config.ts
default defineConfig(,
})
```

Start both the backend and frontend:

**Terminal 1**: Start the backend

```sh Backend
npx tsx --watch backend/server.ts
```

**Terminal 2**: Start the frontend

```sh Frontend  
npx vite
```

Open `http://localhost:5173` in your browser. Try opening multiple tabs to see real-time sync in action.

By default, Rivet stores actor state on the local file system and will not scale in production.

The following providers let you deploy & scale Rivet:

For production with Redis storage, install the Redis driver:

```sh
npm install @rivetkit/redis
```

Then configure the driver:

```ts server.ts
const  = registry.createServer();

// ... rest of server setup ...
```

Your backend can now be deployed to your cloud provider of choice.

Deploy to Cloudflare Workers, install the Cloudflare Workers driver:

```sh
npm install @rivetkit/cloudflare-workers
```

Update your `server.ts` to support Cloudflare Workers:

  ```ts server.ts
  const  = createServerHandler(registry);
  ;
  ```

Update your configuration file to support `ACTOR_DO` and `ACTOR_KV` bindings:

```json wrangler.json

  ],
  "durable_objects": 
    ]
  },
  "kv_namespaces": [
    
  ]
}
```

Finally, deploy:

```sh
wrangler deploy
```

## Configuration Options

### Add Your Own Backend Endpoints

Add custom HTTP endpoints alongside your actors to handle additional business logic, authentication, and integrations with external services.

See [backend quickstart](/docs/actors/quickstart/backend) for more information.