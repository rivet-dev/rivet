# ip-info

Fetches and caches IP info from ipinfo.io and returns a friendly datastructure with any needed information.

If the provided IP address is bogon or anycast, we return `None` since we can't get information about this IP.
