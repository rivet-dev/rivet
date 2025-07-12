---
name = "REQUEST_QUERY_FIELD_NOT_GROUPABLE"
description = "Field cannot be used in GROUP BY."
http_status = 400
---

# Field Not Groupable

The specified field exists but cannot be used in GROUP BY operations.

## Causes

- Field is not marked as groupable in the schema
- Field type doesn't support grouping
- Using a computed or derived field that can't be grouped