// Cover artwork for the cookbook index cards. Every image is a public-domain
// or CC0 artwork hosted on the rivet-assets R2 bucket. Sourcing and license
// details for each work (and verified alternates) are documented in the
// research note "cookbook-cover-image-candidates".
//
// The crop parameters are tuned per artwork so each card centers on the
// painting's focal subject at the tall 5:7 aspect ratio. transform plus
// transformOrigin zooms into a focal point; filter normalizes exposure so the
// covers sit at an even darkness on the black page.

export interface CookbookCoverArt {
	// Artwork title, artist, and date, kept for reference.
	artwork: string;
	src: string;
	width: number;
	height: number;
	objectPosition?: string;
	transform?: string;
	transformOrigin?: string;
	filter?: string;
}

export const cookbookCovers: Record<string, CookbookCoverArt> = {
	"ai-agent": {
		artwork: "The Thinker, Auguste Rodin, modeled 1880",
		src: "https://assets.rivet.dev/website/images/thinking/thinker.jpg",
		width: 3140,
		height: 4000,
		objectPosition: "50% 28%",
	},
	"ai-agent-workspace": {
		artwork:
			"The Alchymist Discovering Phosphorus, William Pether after Joseph Wright of Derby, 1771",
		src: "https://assets.rivet.dev/website/images/cookbook/alchymist-discovering-phosphorus.jpg",
		width: 2679,
		height: 3400,
		transform: "scale(1.55)",
		transformOrigin: "26% 74%",
		filter: "brightness(1.18)",
	},
	"chat-room": {
		artwork: "Merry Company on a Terrace, Jan Steen, ca. 1670",
		src: "https://assets.rivet.dev/website/images/cookbook/merry-company-on-a-terrace.jpg",
		width: 3556,
		height: 3829,
		transform: "scale(1.12)",
		transformOrigin: "45% 42%",
		filter: "brightness(0.95)",
	},
	"collaborative-text-editor": {
		artwork: "Saint Matthew Writing His Gospel, Carlo Dolci, 1640s",
		src: "https://assets.rivet.dev/website/images/cookbook/saint-matthew-writing-his-gospel.jpg",
		width: 2400,
		height: 2897,
		objectPosition: "45% 22%",
		transform: "scale(1.12)",
	},
	"cron-jobs": {
		artwork: "The November Meteors, Etienne Leopold Trouvelot, 1881-82",
		src: "https://assets.rivet.dev/website/images/cookbook/november-meteors.jpg",
		width: 1994,
		height: 2560,
		objectPosition: "50% 42%",
		transform: "scale(1.22)",
	},
	"live-cursors": {
		artwork: "Curiosity, Gerard ter Borch the Younger, ca. 1660-62",
		src: "https://assets.rivet.dev/website/images/thinking/think.jpg",
		width: 2935,
		height: 3638,
		objectPosition: "50% 30%",
	},
	"multiplayer-game": {
		artwork: "The Card Players, Paul Cezanne, 1890-92",
		src: "https://assets.rivet.dev/website/images/cookbook/the-card-players.jpg",
		width: 3909,
		height: 3112,
		objectPosition: "50% 35%",
	},
	"per-tenant-database": {
		artwork: "Dolls' house of Petronella Oortman, c. 1686-1710",
		src: "https://assets.rivet.dev/website/images/cookbook/dollhouse-petronella-oortman.jpg",
		width: 2400,
		height: 3054,
		objectPosition: "50% 40%",
		filter: "brightness(0.95)",
	},
};
