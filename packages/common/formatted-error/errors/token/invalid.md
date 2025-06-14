---
name = "TOKEN_INVALID"
description = "Token is invalid: {reason}."
description_basic = "Token is invalid."
http_status = 400
---

# Token Invalid

The given token could not be parsed.

## Token is invalid: invalid signature

The given token does not match the official Rivet signature for signing tokens. If you are using the Rivet
CLI, ensure that the token you are using belongs to the same cluster that it was created from.

## Token is invalid: invalid separator count

The given token has an invalid amount of separators (`.`). Inspect your token to make sure it has either 2 or
3 segments separated by `.`'s. This can be caused by improperly copying and pasting the token when using it.

## Token is invalid: invalid algorithm

The algorithm specified in the token is not allowed by Rivet. This likely signified a breaking change with how
tokens are parsed and should never show up.

