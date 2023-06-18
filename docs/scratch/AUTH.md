# Auth

Rivet uses JWT for managing authentication.

See [token-create](svc/token-create/src/main.rs) and ![auth.rs](svc/api-auth/src/route/auth.rs) for more details.

## Entitlements

Entitlements define what a token can do. A token can have multiple entitlements.

## Issuer

Token issuers are formatted as `gg.rivet.svc.[SERVICE_NAME]`.

## Expirations

-   A user auth token expires after 15 minutes.
-   A refresh token expires after 90 days.

## Security

-   A refresh token is stored in a cookie so it cannot be accessed from JS.
-   The client will fetch a temporary token from the auth server (which reads the refresh token from the cookie) and use that for any subsequent requests.
-   The client will automatically fetch a new token from the auth server when the old token expires.

## Hub Authentication

There are two important tokens for user auth:


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
