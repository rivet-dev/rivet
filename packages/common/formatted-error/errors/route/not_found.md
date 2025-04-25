---
name = "ROUTE_NOT_FOUND"
description = "The requested route could not be found."
description_basic = "Route not found."
http_status = 404
---

# not_found

The requested route could not be found.

### Details

This error occurs when attempting to access, update, or delete a route that does not exist or has been deleted. Routes are identified by either their UUID or by a combination of namespace ID and name ID.

### Examples

This error might occur when:
- The route ID provided is invalid or does not exist
- The namespace and name ID combination does not match any active route
- The route has been deleted (has a delete_ts timestamp)