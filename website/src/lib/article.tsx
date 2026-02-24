import forestAnderson from "@/authors/forest-anderson/avatar.jpeg";
import nathanFlurry from "@/authors/nathan-flurry/avatar.jpeg";
import nicholasKissel from "@/authors/nicholas-kissel/avatar.jpeg";

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
