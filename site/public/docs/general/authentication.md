# Authentication

Secure your actors with authentication and authorization

Rivet provides multiple authentication methods to secure your actors. Use `onAuth` for server-side validation or `onBeforeConnect` for actor-level authentication.

## Authentication Methods

### onAuth Hook (Recommended)

The `onAuth` hook runs on the HTTP server before clients can access actors. This is the preferred method for most authentication scenarios.

```typescript
const chatRoom = actor( = opts;
    
    // Extract token from params or headers
    const token = params.authToken || req.headers.get("Authorization");
    
    if (!token) 
    
    // Validate token and return user data
    const user = await validateJWT(token);
    return ;
  },
  
  state: ,
  
  actions:  = c.conn.auth;
      
      if (role !== "member") 
      
      const message = ;
      
      c.state.messages.push(message);
      c.broadcast("newMessage", message);
      return message;
    }
  }
});
```

### `onBeforeConnect` Hook

Use `onBeforeConnect` when you need access to actor state for authentication:

```typescript
const userProfileActor = actor(),
  
  state: ,
  
  onBeforeConnect: async (c, opts) =>  = opts;
    const userId = await validateUser(params.token);
    
    // Check if user can access this profile
    if (c.state.isPrivate && c.state.ownerId !== userId) 
  },
  
  createConnState: (c, opts) => ;
  },
  
  actions: 
      
      // Update profile...
    }
  }
});
```

Prefer `onAuth` over `onBeforeConnect` when possible, as `onAuth` runs on the HTTP server and uses fewer actor resources.

## Connection Parameters

Pass authentication data when connecting:

```typescript
// Client side
const chat = client.chatRoom.getOrCreate(["general"]);
const connection = chat.connect();

// Or with action calls
const counter = client.counter.getOrCreate(["user-counter"], );
```

## Intent-Based Authentication (Experimental)

The `onAuth` hook receives an `intents` parameter indicating what the client wants to do:

```typescript
const secureActor = actor( = opts;
    
    // Different validation based on intent
    if (intents.has("action"))  else if (intents.has("connect")) 
    
    throw new UserError("Unknown intent");
  },
  
  actions: 
  }
});
```

## Error Handling

### Authentication Errors

Use specific error types for different authentication failures:

```typescript
const protectedActor = actor(
    
    try  catch (error) 
      throw new Unauthorized("Invalid authentication token");
    }
  },
  
  actions: 
      return "Admin content";
    }
  }
});
```

### Client Error Handling

Handle authentication errors on the client:

```typescript
try  catch (error)  else if (error.code === "FORBIDDEN") 
}
```

## Integration with Auth Providers

### Better Auth Integration

  Complete integration guide for Better Auth

### JWT Authentication

```typescript
const jwtActor = actor(
    
    try ;
    } catch (error) 
  },
  
  actions:  = c.conn.auth;
      
      if (!permissions.includes("write")) 
      
      // Perform action...
      return ;
    }
  }
});
```

### API Key Authentication

```typescript
const apiActor = actor(
    
    // Validate with your API service
    const response = await fetch(`$/validate`, 
    });
    
    if (!response.ok) 
    
    const user = await response.json();
    return ;
  },
  
  actions: 
      
      return "Premium content";
    }
  }
});
```

## Role-Based Access Control

Implement RBAC with helper functions:

```typescript
// auth-helpers.ts
function requireRole(requiredRole: string) ;
    
    if (roleHierarchy[userRole]  ' required`);
    }
  };
}

// usage in actor
const forumActor = actor(,
  
  actions: ,
    
    editPost: (c, postId: string, content: string) => 
  }
});
```

## Testing Authentication

Mock authentication for testing:

```typescript
// test helpers
function createMockAuth(userData: any) ;
}

// in tests
describe("Protected Actor", () => )
    };
    
    const result = await mockActor.adminOnly();
    expect(result).toBe("Admin content");
  });
  
  it("denies non-admin actions", async () => )
    };
    
    await expect(mockActor.adminOnly()).rejects.toThrow("Admin access required");
  });
});
```

## Best Practices

1. **Use onAuth**: Prefer `onAuth` over `onBeforeConnect` for most authentication
2. **Validate Early**: Authenticate at the HTTP server level when possible
3. **Specific Errors**: Use appropriate error types (Unauthorized, Forbidden)
4. **Rate Limiting**: Consider rate limiting in your authentication logic
5. **Token Refresh**: Handle token expiration gracefully on the client
6. **Audit Logging**: Log authentication events for security monitoring
7. **Least Privilege**: Only grant the minimum permissions needed