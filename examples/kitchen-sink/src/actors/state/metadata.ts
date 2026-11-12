import { actor } from "rivetkit";

type MetadataVars = {
	lastMetadata: any;
	actorName: string;
	storedTags: Record<string, string>;
	storedRegion: string | null;
};

// Note: For testing only - metadata API will need to be mocked
// in tests since this is implementation-specific
export const metadataActor = actor({
	createVars: (c): MetadataVars => ({
		lastMetadata: null,
		// Store the actor name during initialization
		actorName: c.name,
		// Store tags and region in vars for testing since they may not be
		// available in the context in all environments
		storedTags: {},
		storedRegion: null,
	}),
	actions: {
		// Set up test tags - this will be called by tests to simulate tags
		setupTestTags: (c, tags: Record<string, string>) => {
			c.vars.storedTags = tags;
			return tags;
		},

		// Set up test region - this will be called by tests to simulate region
		setupTestRegion: (c, region: string) => {
			c.vars.storedRegion = region;
			return region;
		},

		// Get all available metadata
		getMetadata: (c) => {
			// Create metadata object from stored values
			const metadata = {
				name: c.name,
				tags: c.vars.storedTags,
				region: c.vars.storedRegion,
			};

			// Store for later inspection
			c.vars.lastMetadata = metadata;
			return metadata;
		},

		// Get the actor name
		getActorName: (c) => {
			return c.name;
		},

		// Get a specific tag by key
		getTag: (c, key: string) => {
			return c.vars.storedTags[key] || null;
		},

		// Get all tags
		getTags: (c) => {
			return c.vars.storedTags;
		},

		// Get the region
		getRegion: (c) => {
			return c.vars.storedRegion;
		},

		// Get the stored actor name (from createVars)
		getStoredActorName: (c) => {
			return c.vars.actorName;
		},

		// Get last retrieved metadata
		getLastMetadata: (c) => {
			return c.vars.lastMetadata;
		},
	},
});
