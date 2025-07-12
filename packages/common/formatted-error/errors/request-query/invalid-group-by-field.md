---
name = "REQUEST_QUERY_INVALID_GROUP_BY_FIELD"
description = "Invalid group by field specified."
http_status = 400
---

# Invalid Group By Field

The specified field does not exist in the schema and cannot be used for grouping.

## Causes

- Field name is misspelled
- Field doesn't exist in the schema
- Using a field that's not available for grouping