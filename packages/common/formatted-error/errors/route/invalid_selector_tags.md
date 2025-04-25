---
name = "ROUTE_INVALID_SELECTOR_TAGS"
description = "The selector_tags provided for the route are invalid."
description_basic = "The route selector tags format is invalid."
http_status = 400
---

# invalid_selector_tags

The selector_tags provided for the route are invalid. Route selector tags must follow specific validation rules.

### Details

Route selector tags must meet the following requirements:
- Maximum of 8 key-value pairs
- Keys cannot be empty and must be 32 bytes or less
- Values cannot be empty and must be 1024 bytes or less

Selector tags are used to match routes to specific actors or services based on their tags. Having proper validation ensures reliable routing.

### Examples

Invalid:
- More than 8 key-value pairs
- Empty key: `{"": "value"}`
- Key too long: `{"this_key_is_way_too_long_and_exceeds_the_32_byte_limit": "value"}`
- Empty value: `{"key": ""}`
- Value too long: A value exceeding 1024 bytes

Valid:
- `{"version": "v1"}`
- `{"service": "auth", "environment": "prod"}`