const forestAnderson = { src: "https://assets.rivet.dev/repo/website/src/authors/forest-anderson/avatar.jpeg", width: 256, height: 256, format: "jpg" };
const nathanFlurry = { src: "https://assets.rivet.dev/repo/website/src/authors/nathan-flurry/avatar.jpeg", width: 1516, height: 1516, format: "jpg" };
const nicholasKissel = { src: "https://assets.rivet.dev/repo/website/src/authors/nicholas-kissel/avatar.jpeg", width: 256, height: 256, format: "jpg" };
const andrewTheberge = { src: "https://assets.rivet.dev/repo/website/src/authors/nathan-flurry/avatar.jpeg", width: 1516, height: 1516, format: "jpg" };
export const AUTHORS = {
	"nathan-flurry": {
		name: "Nathan Flurry",
		role: "Co-founder & CTO",
		avatar: nathanFlurry,
		socials: {
			twitter: "https://x.com/NathanFlurry/",
			github: "https://github.com/nathanflurry",
		},
	},
	"nicholas-kissel": {
		name: "Nicholas Kissel",
		role: "Co-founder & CEO",
		avatar: nicholasKissel,
		socials: {
			twitter: "https://x.com/NicholasKissel",
			github: "https://github.com/nicholaskissel",
			bluesky: "https://bsky.app/profile/nicholaskissel.com",
		},
	},
	"forest-anderson": {
		name: "Forest Anderson",
		role: "Founding Engineer",
		avatar: forestAnderson,
		url: "https://twitter.com/angelonfira",
	},
	"andrew-theberge": {
		name: "Andrew Theberge",
		role: "Engineer",
		avatar: andrewTheberge,
	},
} as const;

export const CATEGORIES = {
	changelog: {
		name: "Changelog",
	},
	"monthly-update": {
		name: "Monthly Update",
	},
	"launch-week": {
		name: "Launch Week",
	},
	technical: {
		name: "Technical",
	},
	guide: {
		name: "Guide",
	},
	frogs: {
		name: "Frogs",
	},
} as const;

export type AuthorId = keyof typeof AUTHORS;
export type CategoryId = keyof typeof CATEGORIES;
