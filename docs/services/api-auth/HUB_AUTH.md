# Hub Authentication

There are two important tokens for user authentication in the hub.

**Refresh token**

-   Expires after 90 days
-   Includes a _refresh_ entitlement that includes the user entitlement to generate
-   Single-use and will be regenerated upon consumption with a new expiration
-   Will update the stored session data every time it is used
-   Stored as an `HttpOnly` cookie

**User token**

-   Expires after 15 minutes
-   Includes a _user_ entitlement for the given user
-   Generated using the refresh token above