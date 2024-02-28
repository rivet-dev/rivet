---
name = "MATCHMAKER_CUSTOM_LOBBY_CONFIG_INVALID"
description = "The given custom lobby config is invalid: {reason}"
description_basic = "The given custom lobby config is invalid."
http_status = 400
---

# Matchmaker Custom Lobby Config Invalid

The given custom lobby config is invalid. This is most likely because it exceeds the maximum size limit of 16KiB.

## Publicity errors

You may encounter an error such as `"public" publicity not allowed with this custom game mode`. This occurs when the publicity of the `/create` request is not allowed by the custom game mode config configured for this game.

Given this `rivet.yaml`:

```yaml
matchmaker:
    game_modes:
        default:
            actions:
                create:
                    enabled: true
                    enable_public: false # Optional
                    enable_private: true # Optional
```

The following request will not work because `enable_public` is `false`:

```js
import { RivetClient } from "@rivet-gg/api";
const RIVET = new RivetClient({ token: addYourTokenHere });

// Make request
await RIVET.matchmaker.lobbies.create({
	gameMode: "default",
	publicity: "public",
});
```

Read more about custom game configs [here](https://rivet.gg/docs/matchmaker/guides/custom-games).
