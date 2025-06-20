---
name = "ROUTE_INVALID_NAME_ID"
description = "The name_id provided for the route is invalid."
description_basic = "The route name_id format is invalid."
http_status = 400
---

# Invalid Name ID

The name_id provided for the route is invalid. Route name_ids must be lowercase alphanumeric or dashes without repeating double dashes, and are limited to 16 characters.

### Details

Name IDs are used to uniquely identify routes within a namespace and must follow specific formatting rules. 

Make sure your name_id:
- Contains only lowercase letters (a-z), numbers (0-9), and hyphens (-)
- Does not contain consecutive hyphens (--)
- Does not start or end with a hyphen
- Is 16 characters or less

### Examples

Invalid: `UPPERCASE-route`, `route--name`, `-route-name`, `route-name-`
Valid: `my-route`, `route-123`, `api-v2`
