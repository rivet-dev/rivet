# Idle Lobbies

-   Idle lobbies are updated in `mm-lobby-idle-update`

## When idle lobbies are added

-   `mm-lobby-create` if there are no preemptive players

## When idle lobbies are removed

-   `mm-lobby-find` (on player create)
-   `mm-player-remove` if lobby is empty
