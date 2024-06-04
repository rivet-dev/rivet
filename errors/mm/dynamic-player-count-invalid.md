---
name = "MATCHMAKER_DYNAMIC_MAX_PLAYERS_INVALID"
description = "The given max player count is invalid for this game mode: {game_mode} (max {max})"
description_basic = "The given max player count is invalid."
http_status = 400
---

# Matchmaker Dynamic Max Players Invalid

The given max player count is invalid for the given game mode. This is usually because the player count is
below 1 or above the maximum specified in the game mode config.
