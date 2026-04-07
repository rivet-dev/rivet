import { actor } from "rivetkit";

// Note: For testing only - metadata API will need to be mocked
// in tests since this is implementation-specific
export const metadataActor = actor({
	state: {
		lastMetadata: null as any,
		actorName: "",
		// Store tags and region in state for testing since they may not be
		// available in the context in all environments
		storedTags: {} as Record<string, string>,
		storedRegion: null as string | null,
	},
	onWake: (c) => {
		// Store the actor name during initialization
		c.state.actorName = c.name;
	},
	actions: {
		// Set up test tags - this will be called by tests to simulate tags
		setupTestTags: (c, tags: Record<string, string>) => {
			c.state.storedTags = tags;
			return tags;
		},

		// Set up test region - this will be called by tests to simulate region
		setupTestRegion: (c, region: string) => {
			c.state.storedRegion = region;
			return region;
		},

		// Get all available metadata
		getMetadata: (c) => {
			// Create metadata object from stored values
			const metadata = {
				name: c.name,
				tags: c.state.storedTags,
				region: c.state.storedRegion,
			};

			// Store for later inspection
			c.state.lastMetadata = metadata;
			return metadata;
		},

		// Get the actor name
		getActorName: (c) => {
			return c.name;
		},

		// Get a specific tag by key
		getTag: (c, key: string) => {
			return c.state.storedTags[key] || null;
		},

		// Get all tags
		getTags: (c) => {
			return c.state.storedTags;
		},

		// Get the region
		getRegion: (c) => {
			return c.state.storedRegion;
		},

		// Get the stored actor name (from onWake)
		getStoredActorName: (c) => {
			return c.state.actorName;
		},

		// Get last retrieved metadata
		getLastMetadata: (c) => {
			return c.state.lastMetadata;
		},
	},
});
