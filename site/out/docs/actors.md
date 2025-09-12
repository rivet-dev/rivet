# Overview

Actors for long-lived processes with durable state, realtime, and hibernate when not in use.

## Quickstart

  Set up actors with Node.js, Bun, and web frameworks

  Build real-time React applications with actors

## Features

- **Long-Lived, Stateful Compute**: Each unit of compute is like a tiny server that remembers things between requests – no need to re-fetch data from a database or worry about timeouts. Like AWS Lambda, but with memory and no timeouts.

- **Blazing-Fast Reads & Writes**: State is stored on the same machine as your compute, so reads and writes are ultra-fast. No database round trips, no latency spikes.

- **Realtime, Made Simple**: Update state and broadcast changes in realtime with WebSockets or SSE. No external pub/sub systems, no polling – just built-in low-latency events.

- **Store Data Near Your Users**: Your state lives close to your users on the edge – not in a faraway data center – so every interaction feels instant. (Not all platforms supported.)

- **Infinitely Scalable**: Automatically scale from zero to millions of concurrent actors. Pay only for what you use with instant scaling and no cold starts.

- **Fault Tolerant**: Built-in error handling and recovery. Actors automatically restart on failure while preserving state integrity and continuing operations.

- **Type Safety**: End-to-end TypeScript safety between clients and actors with full type inference and compile-time checking.

## Use Cases

Actors are perfect for applications that need persistent state and real-time updates:

- **AI & Automation**
  - **AI agents**: Stateful AI assistants with conversation history
  - **AI sandbox orchestration**: Actors can orchestrate logic running inside of agents' sandboxes
  - **Durable workflows**: Long-running business processes with state persistence and recovery

- **Real-time Communication**
  - **Chat rooms**: Real-time messaging with message history and user presence
  - **Collaborative documents**: Multiple users editing documents simultaneously (Yjs integration)
  - **Multiplayer games**: Game state management with real-time updates
  - **Live events**: Broadcasting updates to many participants

- **Data & Synchronization**
  - **Local-first sync**: Offline-first applications with server synchronization
  - **Multi-tenant databases**: Isolated data stores for each user or tenant
  - **Scheduling**: Time-based task execution with persistent state

- **Infrastructure**
  - **Rate limiting**: Distributed rate limiting with persistent counters
  - **Stream processing**: Real-time data processing with persistent state

## Core Concepts

### State Management

Actors maintain persistent state that survives restarts, crashes, and deployments. State can be defined as a constant or created dynamically:

```typescript
const counter = actor(,
  
  actions: ,
    
    getCount: (c) => c.state.count,
  }
});
```

Learn more about [state management](/docs/actors/state).

### Actions

Actions are the primary way to interact with actors. They're type-safe functions that can modify state and communicate with clients:

```typescript
const chatRoom = actor(,
  
  actions: ;
      c.state.messages.push(message);
      c.broadcast("newMessage", message);
      return message;
    },
    
    getMessages: (c) => c.state.messages
  }
});
```

Actions can be called from your backend, your clients, or other actors:

```typescript
const room = client.chatRoom.getOrCreate(["general"]);
const message = await room.sendMessage("user-123", "Hello everyone!");
```

Learn more about [actions](/docs/actors/actions) and [communicating with actors](/docs/actors/communicating-between-actors).

### Real-time Communication & Events

Actors support real-time bidirectional communication through WebSocket and SSE connections. Clients can establish persistent connections to receive live updates.

For example, to send events to all connected clients:

```typescript
const liveAuction = actor(,
  
  actions: );
      return amount;
    }
  }
});
```

Clients connect and listen for real-time updates:

```typescript
const auction = client.liveAuction.getOrCreate(["auction-123"]);
const connection = auction.connect();

connection.on("newBid", (data) => `);
});

await auction.placeBid(150);
```

Learn more about [events](/docs/actors/events) and [client communication](/docs/actors/communicating-between-actors).

### Scheduling & Lifecycle

Actors support scheduled tasks and lifecycle management:

```typescript
const reminder = actor(,
  
  actions: ,
    
    sendReminder: (c) => );
    }
  }
});
```

Learn more about [actor lifecycle](/docs/actors/lifecycle).

### Type Safety

Rivet provides end-to-end TypeScript safety between clients and actors:

```typescript Actor
const userManager = actor( as Record },
  
  actions: ;
      return ;
    },
    
    getUser: (c, userId: string) => c.state.users[userId]
  }
});
```

```typescript Client
const manager = client.userManager.getOrCreate(["default"]);

const user = await manager.createUser("Alice");
// Type: 

const foundUser = await manager.getUser(user.userId);
// Type:  | undefined
```

## Frequently Asked Questions

        Some software makes sense to separate – e.g., for data lakes or highly relational data. But at the end of the day, data has to be partitioned somewhere at some point.

        Usually "faster" databases like Cassandra, DynamoDB, or Vitess make consistency tradeoffs to get better performance. Stateful serverless forces you to think about how your data is sharded for better performance, better scalability, and less consistency footguns.

        See [Sharing and Joining State](/docs/actors/sharing-and-joining-state) for detailed strategies on combining data from multiple actors.

        OLAP, data lakes, graph databases, and highly relational data are currently not ideal use cases for the actor model.

        Yes, but only in the same way that storing data in a single database row creates a bottleneck.

		Just like a single database row can cause contention when multiple clients try to read and write the same data, a single actor can become a bottleneck if too many requests target it.

		The solution is the same: shard your data across multiple actors to distribute the load and scale seamlessly.

		However, actors handle much higher throughput than traditional database rows because they keep data in memory, making read and write operations significantly faster.