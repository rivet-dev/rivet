---
name = "REQUEST_QUERY_FIELD_NOT_MAP"
description = "Field is not a map type."
http_status = 400
---

# Field Not Map

The specified field is not a map type but a map key was provided.

## Causes

- Trying to access a map key on a non-map field
- Field type mismatch in query
- Incorrect field path syntax