# Overview

Actors are lightweight, stateful functions that maintain persistent state, provide real-time communication, and hibernate when not in use.

## Quickstart

  Set up actors with Node.js, Bun, and web frameworks

  Build real-time React applications with actors

## Key Features

- **Long-Lived, Stateful Compute**: Each unit of compute is like a tiny server that remembers things between requests – no need to reload data or worry about timeouts. Like AWS Lambda, but with memory and no timeouts.

- **Blazing-Fast Reads & Writes**: State is stored on the same machine as your compute, so reads and writes are ultra-fast. No database round trips, no latency spikes.

- **Realtime, Made Simple**: Update state and broadcast changes in realtime with WebSockets or SSE. No external pub/sub systems, no polling – just built-in low-latency events.

## Use Cases

Actors are perfect for applications that need persistent state and real-time updates:

### AI & Automation
- **AI agents**: Stateful AI assistants with conversation history
- **Workflow automation**: Long-running business processes with state persistence

### Real-time Communication
- **Collaborative documents**: Multiple users editing documents simultaneously
- **Multiplayer games**: Game state management with real-time updates
- **Chat rooms**: Real-time messaging with message history and user presence
- **Live events**: Broadcasting updates to many participants

### Data & Synchronization
- **Local-first sync**: Offline-first applications with server synchronization
- **Per-user databases**: Isolated data stores for each user or tenant

### Infrastructure
- **Rate limiting**: Distributed rate limiting with persistent counters
- **Stream processing**: Real-time data processing with persistent state

## State Management

Actors maintain persistent state that survives restarts, crashes, and deployments. State can be defined as a constant or created dynamically:

```typescript
const counter = actor(,
  
  actions: ,
    
    getCount: (c) => c.state.count,
  }
});
```

Learn more about [state management](/docs/actors/state).

## Actions

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

Learn more about [actions](/docs/actors/actions) and [communicating with actors](/docs/actors/communicating-with-actors).

## Real-time Communication

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

Learn more about [events](/docs/actors/events) and [client communication](/docs/actors/communicating-with-actors).

## Scheduling & Lifecycle

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

## Type Safety

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

        Rivet is a framework written in TypeScript that provides high-level functionality. Rivet is an open-source serverless platform written in Rust with features tailored for stateful serverless.

        You can think of it as Rivet is to Rivet as Next.js is to Vercel.

        While Rivet is the primary maintainer of Rivet, we intend for this to be community driven.

        Stateful serverless is very similar to actors: it's essentially actors with persistence, and usually doesn't have as rigid constraints on message handling. This makes it more flexible while maintaining the core benefits of the actor model.

        Stateless serverless works well when you have an external resource that maintains state. Stateful serverless, on the other hand, is almost like a mini-database.

        Sometimes it makes sense to use stateless serverless to make requests to multiple stateful serverless instances, orchestrating complex operations across multiple state boundaries.

        By storing state in memory and flushing to a persistence layer, we can serve requests instantly instead of waiting for a round trip to the database. There are additional optimizations that can be made around your state to tune the durability of it.

        Additionally, data can be stored near your users at the edge, ensuring round-trip times of less than 50ms when they request it. This edge-first approach eliminates the latency typically associated with centralized databases.

        Some software makes sense to separate – e.g., for data lakes or highly relational data. But at the end of the day, data has to be partitioned somewhere at some point.

        Usually "faster" databases like Cassandra, DynamoDB, or Vitess make consistency tradeoffs to get better performance. Stateful serverless forces you to think about how your data is sharded for better performance, better scalability, and less consistency footguns.

        OLAP, data lakes, graph databases, and highly relational data are currently not ideal use cases for stateful serverless, though it will get better at handling these use cases over time.

        Yes, but only as much as storing data in a single database row does. We're working on building out read replicas to allow you to perform read-only actions on actors.