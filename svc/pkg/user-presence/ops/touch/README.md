# user-presence-touch

Called any time the user makes a presence-related API call.

This will update the last known timestamp of when the user was seen. We use this to set the user as invisible if they don't make contact for a while.
