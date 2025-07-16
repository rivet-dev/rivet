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

## Example

Here's a basic example of how you might set up a connection to a PostgreSQL database using the `pg` library:

```typescript actor.ts
// Create a database connection pool
const pool = new Pool();

// Create the actor
const databaseActor = actor(,
  
  // Initialize any resources
  onStart: (c) => ,
  
  // Clean up resources if needed
  onShutdown: async (c) => ,
  
  // Define actions
  actions:  catch (error) 
    },
    
    // Example action to insert data into database
    insertData: async (c, data) => ;
      } catch (error) 
    }
  }
});

default databaseActor;
```

## With Drizzle ORM

Here's an example using Drizzle ORM for more type-safe database operations:

```typescript actor.ts
// Define your schema
const users = pgTable("users", );

// Create a database connection
const pool = new Pool();

// Initialize Drizzle with the pool
const db = drizzle(pool);

// Create the actor
const userActor = actor(
  },
  
  actions: 
      
      // Query the database
      const result = await db.select().from(users).where(eq(users.id, userId));
      
      if (result.length === 0)  not found`);
      }
      
      // Cache the result
      c.state.userCache[userId] = result[0];
      return result[0];
    },
    
    // Create a new user
    createUser: async (c, userData) => ).returning();
      
      // Broadcast the new user event
      c.broadcast("userCreated", result[0]);
      
      return result[0];
    }
  }
});

default userActor;
```