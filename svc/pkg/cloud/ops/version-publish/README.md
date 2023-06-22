# cloud-version-publish

This service does three main things:

1. Produce the metadata for the given config for each cloud service. This will also validate that the config is valid and return an error if something is wrong.
    - It is the responsibility of the client to manually validate configs at the moment. Invalid configs should be considered internal errors that the client did not catch before sending the request.
2. Create a new game version.
3. Publish each cloud service's config corresponding with the game version.

## Prepare step

This should not write anything to the database pertaining to this version. This should only allocate the appropriate resources in order to create the version. This should function as if the public function may never be called.

## Publish step

-   This should write the new version configs to the database.
