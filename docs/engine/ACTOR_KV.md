# Actor KV Storage

Each actor has its own private KV store which can be manipulated or accessed with the various provided KV operations.

## Keys and Values

A KV key is a byte array, aka a blob. A KV value is also a byte array/blob.

Every set KV value contains metadata which includes the version and create timestamp of the key (version being a string byte array denoting the version of the Rivet Engine).

## Operations

### Get

- Input
	- List of keys
- Output
	- List of keys
	- List of values
	- List of metadata

Keys that don't exist aren't included in the output so it is important to read the output's list of keys.

### List

- Input
	- Query mode
		- All - Lists all keys up to the given limit
		- Range - Lists all keys between the two given keys (exclusivity toggleable)
		- Prefix - Lists all keys with the given key as a prefix
	- Reverse - Whether to iterate keys in descending order instead of ascending
	- Limit - how maximum returned keys
- Output
	- List of keys
	- List of values
	- List of metadata

### Put

- Input
	- List of keys
	- List of values
- Output
	- Empty

### Delete

- Input
	- List of keys
- Output
	- Empty

### Drop

- Input
	- Empty
- Output
	- Empty

This operation deletes all keys in the entire actor's KV store. Use cautiously.

## Errors

Every operation can return an error instead of its regular response. The error includes a message string.

## Implementation Details

Each KV request has a u32 request ID which is to be provided by the user (handled internally by RivetKit). Rivet makes no attempt to order or deduplicate the responses to KV requests, it is up to the client to match the responses to the requests via the request ID.
