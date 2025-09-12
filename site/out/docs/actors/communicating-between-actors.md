# Communicating Between Actors

Learn how actors can call other actors and share data

Actors can communicate with each other using the server-side actor client, enabling complex workflows and data sharing between different actor instances.

We recommend reading the [clients documentation](/docs/actors/clients) first. This guide focuses specifically on communication between actors.

## Using the Server-Side Actor Client

The server-side actor client allows actors to call other actors within the same registry. Access it via `c.client()` in your actor context:

```typescript
const orderProcessor = actor(,
  
  actions: );
      
      return ;
    }
  }
});
```

## Use Cases and Patterns

### Actor Orchestration

Use a coordinator actor to manage complex workflows:

```typescript
const workflowActor = actor(,
  
  actions: );
      return result;
    }
  }
});
```

### Data Aggregation

Collect data from multiple actors:

```typescript
const analyticsActor = actor(,
  
  actions: ,
        generatedAt: Date.now()
      };
      
      c.state.reports.push(report);
      return report;
    }
  }
});
```

### Event-Driven Architecture

Use connections to listen for events from other actors:

```typescript
const auditLogActor = actor(,
  
  actions: );
      });
      
      // Listen for order events
      orderActor.on("orderCompleted", (order) => );
      });
      
      return ;
    }
  }
});
```

### Batch Operations

Process multiple items in parallel:

```typescript
// Process items in parallel
const results = await Promise.all(
  items.map(item => client.processor.getOrCreate([item.type]).process(item))
);
```