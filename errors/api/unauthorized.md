---
name = "API_UNAUTHORIZED"
description = "Failed to authenticate: {reason}"
description_basic = "Make sure your `Authorization` header is properly set."
http_status = 401
---

# API Unauthorized

The user is not authorized for the reason given, or the user did not provide a bearer token to authenticate with.

A bearer token is a token in the `Authorization` header that follows the form `Authorization: Bearer <token>`. The prefix, `"Bearer "`, is required. More info: [https://oauth.net/2/access-tokens/](https://oauth.net/2/access-tokens/).
