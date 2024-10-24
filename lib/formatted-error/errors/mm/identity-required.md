---
name = "MATCHMAKER_IDENTITY_REQUIRED"
description = "This resource can not be accessed without an identity."
http_status = 400
---

# Matchmaker Identity Required

This resource can not be accessed without an identity.

If you are a developer seeing this error, make sure your API calls to matchmaker endpoints include a bearer
token with game user entitlements.
