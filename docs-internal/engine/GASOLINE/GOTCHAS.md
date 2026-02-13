# Gotchas

## Signal tags

Internally, it is more efficient to order signal tags in a manner of most unique to least unique:

- Given a workflow with tags:
	- namespace = foo
	- type = normal

The signal should be published with `namespace = foo` first, then `type = normal`
