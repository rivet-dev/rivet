# Mastra

Integrate Rivet with Mastra for AI agents

Mastra provides AI agents with tools. This integration stores agent conversations and tool data in Rivet Actors.

	Check out the complete example

## Installation

Install Mastra alongside Rivet:

```bash
npm install @mastra/core @ai-sdk/openai ai zod hono @hono/node-server @hono/node-ws @hono/zod-openapi @rivetkit/actor dotenv
npm install -D tsx typescript @types/node
```

	This example requires an OpenAI API key. Set it as `OPENAI_API_KEY` in your environment.

## Basic Setup

Set up your Rivet Actor registry:

```typescript src/registry.ts
import  from "./utils/logging.js";

// Load environment variables
dotenv.config();

const aiAgent = actor(,
		toolData: ,
	},

	onAuth: () => ,

	onCreate: (c) => ,

	actions: ;
			}

			const userMessage: Message = ;
			c.state.messages.push(userMessage);

			// Keep conversation history manageable
			if (c.state.messages.length > 50) 

			try ;
				c.state.messages.push(assistantMessage);

				return ;
			} catch (error) `;
				if (
					(error as Error).message &&
					(error as Error).message.includes("quota")
				) 

				const errorResponse: Message = ;
				c.state.messages.push(errorResponse);

				return ;
			}
		},

		getHistory: (c, limit?: number) => ;
		},

		clear: (c) => ;
			c.state.toolData = ;
			logActorCleared(c.name);
			return ;
		},

		getMetadata: (c) => (),
	},
});

const registry = setup(,
});
```

Set up your Mastra agent with tools:

```typescript src/agents/chat-agent.ts
function createChatAgent(state: AgentState, actorName: string) ;
  
  const chatAgent = new Agent(
  });

  return new Mastra(
  });
}
```

Create an HTTP server that uses your persistent AI agents:

```typescript src/server.ts
// Set up Rivet Actor registry
const registry = setup(
});

const  = registry.createServer();
const app = new Hono();

// Chat with AI agent
app.post("/chat", async (c) =>  = await c.req.json();

		// Get or create persistent AI agent for this user
		const actor = await client.aiAgent.getOrCreate([userId]);
		const result = await actor.chat(message);

		return c.json(result);
	} catch (error) , 500);
	}
});

// Get conversation history
app.get("/chat/:userId/history", async (c) =>  = c.req.param();
		const actor = await client.aiAgent.getOrCreate([userId]);
		const result = await actor.getHistory();

		return c.json(result);
	} catch (error)  = c.req.param();
		return c.json();
	}
});

// Clear conversation
app.delete("/chat/:userId", async (c) =>  = c.req.param();
		const actor = await client.aiAgent.getOrCreate([userId]);
		const result = await actor.clear();

		return c.json(result);
	} catch (error) , 500);
	}
});

// Start server with Rivet integration
serve(app);
```

## Usage Examples

```bash
# Start a conversation
curl -X POST http://localhost:8080/chat \
  -H "Content-Type: application/json" \
  -d ''

# Get conversation history  
curl http://localhost:8080/chat/user123/history

# Clear conversation
curl -X DELETE http://localhost:8080/chat/user123
```

The AI agent will remember previous conversations, weather queries, and any information you tell it to remember - all persisted automatically by Rivet Actors.

## Features

- **Persistent Conversations**: Chat history automatically saved and restored
- **Memory Tools**: AI can remember and recall information across sessions
- **Weather Integration**: Real-time weather data for any location
- **Web Interface**: Built-in chat interface for testing (visit http://localhost:8080)
- **Error Handling**: Graceful handling of API errors and quota limits