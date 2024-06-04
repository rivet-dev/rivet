# linode-server-provision

This was meant to be agnostic to all other packages and simply create a server on Linode, but because of
custom API keys and prebake images we need to include a `datacenter_id` in the request. In the future and if
needed this can be made optional so that this endpoint does not require a `datacenter_id`.
