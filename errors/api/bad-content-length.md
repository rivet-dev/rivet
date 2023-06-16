---
name = "API_BAD_CONTENT_LENGTH"
description = "The content length could not be deserialized."
http_status = 400
---

# API Bad Content Length

The Content-Length header or a `content_length` property in the body could not be deserialized. This is most likely due to a negative value being passed or a value out of bounds of a standard signed 64-bit integer.
