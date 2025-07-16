# Better Auth

Integrate Rivet with Better Auth for authentication

Better Auth provides a comprehensive authentication solution that integrates seamlessly with Rivet Actors using the `onAuth` hook.

	Check out the complete example

## Installation

Install Better Auth alongside Rivet:

```bash
npm install better-auth better-sqlite3
npm install -D @types/better-sqlite3

# For React integration
npm install @rivetkit/react
```

	This example uses SQLite to keep the example. In production, replace this with a database like Postgres. Read more about [configuring your database in Better Auth](https://www.better-auth.com/docs/installation#configure-database).

## Backend Setup

Create your authentication configuration:

```typescript auth.ts
const auth = betterAuth(,
});
```

Create and apply the database schema:

```bash
# Generate migration files
pnpm dlx @better-auth/cli@latest generate --config auth.ts

# Apply migrations to create the database tables
pnpm dlx @better-auth/cli@latest migrate --config auth.ts -y
```

Use the `onAuth` hook to validate sessions:

```typescript registry.ts
const chatRoom = actor( = opts;
    
    // Use Better Auth to validate the session
    const authResult = await auth.api.getSession();
    if (!authResult) throw new Unauthorized();
    
    // Return user data to be available in actor
    return ;
  },
  
  state: ,
  
  actions:  = c.conn.auth;
      
      const newMessage = ;
      
      c.state.messages.push(newMessage);
      c.broadcast("newMessage", newMessage);
      
      return newMessage;
    },
    
    getMessages: (c) => c.state.messages,
  },
});

const registry = setup(,
});
```

Configure your server to handle Better Auth routes and Rivet:

```typescript
// server.ts
const  = registry.createServer();
const app = new Hono();

// Configure CORS for Better Auth + Rivet
app.use("*", cors());

// Mount Better Auth routes
app.on(["GET", "POST"], "/api/auth/**", (c) => 
  auth.handler(c.req.raw)
);

// Start Rivet server
serve(app);
```

## Frontend Integration

Create a Better Auth client for your frontend:

```typescript
// auth-client.ts
const authClient = createAuthClient();
```

Create login/signup forms:

```tsx
// AuthForm.tsx
function AuthForm() );
      } else );
      }
    } catch (error) 
  };

  return (

          required
        />
      )}
      
       setEmail(e.target.value)}
        required
      />
      
       setPassword(e.target.value)}
        required
      />

       setIsLogin(!isLogin)}
      >

  );
}
```

Use authenticated sessions with Rivet:

```tsx
// ChatRoom.tsx
const client = createClient("http://localhost:8080");
const  = createRivetKit(client);

interface ChatRoomProps  };
  roomId: string;
}

function ChatRoom(: ChatRoomProps) );

  const sendMessage = async () => ;

  return (

        Welcome, !
         authClient.signOut()}>Sign Out

        : 
          
        ))}

         setNewMessage(e.target.value)}
          onKeyPress=
          placeholder="Type a message..."
        />
        Send

  );
}
```

## Advanced Features

### Role-Based Access

Add role checking to your actors:

```typescript
const adminActor = actor();
    if (!authResult) throw new Unauthorized();
    
    return ;
  },
  
  actions:  = c.conn.auth;
      if (user.role !== "admin") 

      // Admin-only action
      // ... implementation
    },
  },
});
```

### Session Management

Handle session expiration gracefully:

```tsx
// hooks/useAuth.ts
function useAuthWithRefresh()  = authClient.useSession();
  
  useEffect(() => 
  }, [error]);
  
  return session;
}
```

## Production Deployment

For production, you'll need a database from a provider like [Neon](https://neon.tech/), [PlanetScale](https://planetscale.com/), [AWS RDS](https://aws.amazon.com/rds/), or [Google Cloud SQL](https://cloud.google.com/sql).

Configure your production database connection:

```typescript
// auth.ts
const auth = betterAuth(),
  trustedOrigins: [process.env.FRONTEND_URL],
  emailAndPassword: ,
});
```

Set the following environment variables for production:

```bash
DATABASE_URL=postgresql://username:password@localhost:5432/myapp
FRONTEND_URL=https://myapp.com
BETTER_AUTH_SECRET=your-secure-secret-key
BETTER_AUTH_URL=https://api.myapp.com
```

Read more about [configuring Postgres with Better Auth](https://www.better-auth.com/docs/adapters/postgresql).

	Don't forget to re-generate & re-apply your database migrations if you change the database in your Better Auth config.