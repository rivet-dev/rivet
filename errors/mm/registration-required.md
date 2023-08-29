---
name = "MATCHMAKER_REGISTRATION_REQUIRED"
description = "This resource can not be accessed without a registered identity."
http_status = 400
---

# Matchmaker Registration Required

This resource can not be accessed without a registered identity.

If you are a developer seeing this error, make sure your API calls to matchmaker endpoints include a bearer
token with game user entitlements, and the given game user is registered on Rivet.
