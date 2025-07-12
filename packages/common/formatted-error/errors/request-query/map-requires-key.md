---
name = "REQUEST_QUERY_MAP_REQUIRES_KEY"
description = "Map property requires a key for GROUP BY."
http_status = 400
---

# Map Requires Key

Map properties require a map key to be specified when used in GROUP BY.

## Causes

- Using a map field in GROUP BY without specifying a key
- Missing map key in the KeyPath structure
- Attempting to group by the entire map instead of a specific key