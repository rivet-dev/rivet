---
name = "ROUTE_INVALID_PATH"
description = "The path provided for the route is invalid."
description_basic = "The route path format is invalid."
http_status = 400
---

# invalid_path

The path provided for the route is invalid. Route paths must follow specific formatting rules.

### Details

Route paths must meet the following requirements:
- Must start with a forward slash (/)
- Must not end with a forward slash (/) unless it's the root path (/)
- Must not contain consecutive slashes (//)
- Can have at most 8 path components
- Total length must not exceed 256 characters
- Each path component:
  - Must not be empty
  - Must not be longer than 64 characters
  - Must contain only alphanumeric characters, hyphens (-), underscores (_), or dots (.)
  - Must not start or end with a dot
  - Must not contain consecutive dots (..)

### Examples

Invalid:
- `api/v1` (doesn't start with slash)
- `/api/v1/` (ends with slash)
- `/api//v1` (contains consecutive slashes)
- `/api/v1/users/posts/comments/likes/replies/authors/extra` (too many components)
- `/invalid~path` (invalid character)
- `/api/.hidden` (component starts with dot)
- `/api/hidden.` (component ends with dot)
- `/api/config..json` (component contains consecutive dots)
- `/component-name-that-is-way-too-long-and-exceeds-the-maximum-allowed-length-for-a-path-component` (component too long)

Valid:
- `/`
- `/api`
- `/api/v1`
- `/api/v1/users`
- `/api/v1/users/config.json`
- `/api/v1/users/my-service_name`