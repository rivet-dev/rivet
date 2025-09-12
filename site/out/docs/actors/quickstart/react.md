# React Quickstart

Build real-time React applications with Rivet Actors

```sh
npm install @rivetkit/actor @rivetkit/react
```

Create your actor registry on the backend:

```ts }
const counter = actor(,  // Skip authentication (can be configured later)
	state: ,
	actions: ,
		getCount: (c) => c.state.count,
	},
});

const registry = setup(,
});
```

Start a server to run your actors:

```ts }
// Run server with default configuration (port 8080)
registry.runServer();
```

Set up your React application:

```tsx }
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

```ts }
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

## Configuration Options

### Add Your Own Backend Endpoints

Add custom HTTP endpoints alongside your actors to handle additional business logic, authentication, and integrations with external services.

See [backend quickstart](/docs/actors/quickstart/backend) for more information.