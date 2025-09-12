# External SQL Database

While actors can serve as a complete database solution, they can also complement your existing databases. For example, you might use actors to handle frequently-changing data that needs real-time access, while keeping less frequently accessed data in your traditional database.

Actors can be used with common SQL databases, such as PostgreSQL and MySQL.

## Libraries

To facilitate interaction with SQL databases, you can use either ORM libraries or raw SQL drivers. Each has its own use cases and benefits:

-   **ORM Libraries**: Type-safe and easy way to interact with your database

    -   [Drizzle](https://orm.drizzle.team/)
    -   [Prisma](https://www.prisma.io/)

-   **Raw SQL Drivers**: Direct access to the database for more flexibility

    -   [PostgreSQL](https://node-postgres.com/)
    -   [MySQL](https://github.com/mysqljs/mysql)

## Hosting Providers

There are several options for places to host your SQL database:

-   [Supabase](https://supabase.com/)
-   [Neon](https://neon.tech/)
-   [PlanetScale](https://planetscale.com/)
-   [AWS RDS](https://aws.amazon.com/rds/)
-   [Google Cloud SQL](https://cloud.google.com/sql)

## Examples

### Basic PostgreSQL Connection

Here's a basic example of a user actor that creates a database record on start and tracks request counts:

```typescript }
interface ActorInput 

// Create a database connection pool
const pool = new Pool();

// Create the user actor
const userActor = actor() => (),
  
  // Insert user into database when actor starts
  onStart: async (c, opts) => ,
  
  actions: ;
    },
    
    // Get user data
    getUser: async (c) => ;
    }
  }
});

const registry = setup(,
});
```

```typescript }
const client = createClient("http://localhost:8080");

// Create user
const alice = await client.userActor.create("alice", 
});

alice.updateUser("alice2@example.com");

const userData = await alice.getUser();
console.log("User data:", userData);

// Create another user
const bob = await client.userActor.create("bob", 
});
const bobData = await bob.getUser();
```

### Using Drizzle ORM

Here's the same user actor pattern using Drizzle ORM for more type-safe database operations:

```typescript }
interface ActorInput 

// Define your schema
const users = pgTable("users", );

// Create a database connection
const pool = new Pool();

// Initialize Drizzle with the pool
const db = drizzle(pool);

// Create the user actor
const userActor = actor() => (),
  
  // Insert user into database when actor starts
  onStart: async (c, opts) => );
  },
  
  actions: )
        .where(eq(users.username, c.state.username));
      
      return ;
    },
    
    // Get user data
    getUser: async (c) => ;
    }
  }
});

const registry = setup(,
});
```

```typescript }
const client = createClient("http://localhost:8080");

// Create user
const alice = await client.userActor.create("alice", 
});

alice.updateUser("alice2@example.com");

const userData = await alice.getUser();
console.log("User data:", userData);

// Create another user
const bob = await client.userActor.create("bob", 
});
const bobData = await bob.getUser();
```