# Communicating Between Actors

Learn how actors can call other actors and share data

Actors can communicate with each other using the inline client, enabling complex workflows and data sharing between different actor instances.

This guide focuses on communication between actors within the same application. For connecting to actors from client applications, see [Communicating with Actors](/docs/actors/communicating-with-actors).

## Using the Inline Client

The inline client allows actors to call other actors within the same registry. Access it via `c.client()` in your actor actions:

```typescript
const orderProcessor = actor(,
  
  actions: 
      
      // Reserve the stock
      await inventory.reserveStock(order.quantity);
      
      // Process payment through payment actor
      const payment = client.payment.getOrCreate([order.customerId]);
      const result = await payment.processPayment(order.amount);
      
      // Update order state
      c.state.orders.push();
      
      return ;
    }
  }
});
```

## Communication Patterns

The inline client supports the same communication patterns as external clients. See [Communicating with Actors - Actor Handles](/docs/actors/communicating-with-actors#actor-handles) for details on:

- `getOrCreate()` for stateless request-response
- `.connect()` for real-time communication with events
- `get()` and `create()` for explicit actor lifecycle management

## Error Handling

Handle errors gracefully when calling other actors. Error handling works the same as with external clients - see [Communicating with Actors - Error Handling](/docs/actors/communicating-with-actors#error-handling) for details.

```typescript
const orderActor = actor(,
  
  actions: ;
        c.state.orders.push(order);
        return order;
        
      } catch (error) `);
      }
    }
  }
});
```

## Use Cases and Patterns

### Actor Orchestration

Use a coordinator actor to manage complex workflows:

```typescript
const workflowActor = actor(,
  
  actions: ;
    }
  }
});
```

### Data Aggregation

Collect data from multiple actors:

```typescript
const analyticsActor = actor(,
  
  actions: ;
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
      
      return ;
    }
  }
});
```

## Advanced Features

### Type Safety

The inline client maintains full type safety across actor boundaries:

```typescript
const typedActor = actor(
  }
});
```

### Performance Optimization

**Batch Operations**: Process multiple items in parallel:

```typescript
// Process items in parallel
const results = await Promise.all(
  items.map(item => client.processor.getOrCreate([item.type]).process(item))
);
```

**Connection Reuse**: Reuse connections for multiple operations:

```typescript
const connection = client.targetActor.getOrCreate(["shared"]).connect();
try 
} finally 
```

### Testing

Mock the inline client for unit testing:

```typescript
const mockClient = ),
  },
};

// Test with mocked dependencies
const result = await orderProcessor.processOrder.call(
  ,
  orderData
);
```