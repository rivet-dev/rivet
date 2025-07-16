# Logging

Actors provide a built-in way to log complex data to the console.

When dealing with lots of data, `console.log` often doesn't cut it. Using the context's log object (`c.log`) allows you to log complex data using structured logging.

Using the actor logging API is completely optional.

## Log levels

There are 5 log levels:

| Level    | Call                            | Description                                                      |
| -------- | ------------------------------- | ---------------------------------------------------------------- |
| Critical | `c.log.critical(message, ...args);` | Severe errors that prevent core functionality                    |
| Error    | `c.log.error(message, ...args);`    | Errors that affect functionality but allow continued operation   |
| Warning  | `c.log.warn(message, ...args);`     | Potentially harmful situations that should be addressed          |
| Info     | `c.log.info(message, ...args);`     | General information about significant events & state changes     |
| Debug    | `c.log.debug(message, ...args);`    | Detailed debugging information, usually used only in development |

## Structured logging

The built-in logging API (using `c.log`) provides structured logging to let you log key-value
pairs instead of raw strings. Structures logs are readable by both machines &
humans to make them easier to parse & search.

Passing an object to a log will print as structured data. For example:

```typescript
c.log.info('increment', );
// Prints: level=INFO msg=increment connection=123 count=456
```

The first parameter in each log method is the message. The rest of the arguments are used for structured logging.

## `c.log` vs `console.log` logging

`c.log` makes it easier to manage complex logs, while `console.log` can
become unmaintainable at scale.

Consider this example:

```typescript structured_logging.ts
const counter = actor(,
  
  actions: );

      c.state.count += count;
      return c.state.count;
    }
  }
});
```

```typescript unstructured_logging.ts
const counter = actor(,
  
  actions:  with count $`);

      c.state.count += count;
      return c.state.count;
    }
  }
});
```

If you need to search through a lot of logs, it's easier to read the structured logs. To find increments for a single connection, you can search `connection=123`.

Additionally, structured logs can be parsed and queried at scale using tools like Elasticsearch, Loki, or Datadog. For example, you can parse the log `level=INFO msg=increment connection=123 count=456` in to the JSON object `` and then query it as you would any other structured data.

## Usage in lifecycle hooks

The logger is available in all lifecycle hooks:

```typescript
const loggingExample = actor(,
  
  onStart: (c) => );
  },
  
  onBeforeConnect: (c, ) => );
    
    return ;
  },
  
  onConnect: (c) => );
    
    c.state.events.push();
  },
  
  onDisconnect: (c) => );
    
    c.state.events.push();
  },
  
  actions: 
});
```