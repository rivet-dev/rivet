export interface CatalogItem {
	slug: string;
	name: string;
	price: number; // cents
}

export const CATALOG: CatalogItem[] = [
	// Painting a Room
	{ slug: "paint-white", name: "White Paint", price: 2500 },
	{ slug: "paint-blue", name: "Blue Paint", price: 2600 },
	{ slug: "paint-roller", name: "Paint Roller", price: 800 },
	{ slug: "paint-brush", name: "Paint Brush", price: 500 },
	{ slug: "drop-cloth", name: "Drop Cloth", price: 1200 },
	{ slug: "painter-tape", name: "Painter's Tape", price: 300 },
	{ slug: "paint-tray", name: "Paint Tray", price: 400 },
	{ slug: "paint-stirrer", name: "Paint Stirrer", price: 100 },
	{ slug: "paint-primer", name: "Paint Primer", price: 1500 },
	{ slug: "paint-edger", name: "Paint Edger", price: 700 },

	// Installing Laminate Flooring
	{ slug: "laminate-flooring", name: "Laminate Flooring", price: 2000 },
	{ slug: "flooring-underlayment", name: "Flooring Underlayment", price: 1000 },
	{ slug: "flooring-trim", name: "Flooring Trim", price: 1500 },
	{ slug: "flooring-spacer", name: "Flooring Spacer", price: 200 },
	{ slug: "flooring-cutter", name: "Flooring Cutter", price: 3000 },
	{ slug: "flooring-adhesive", name: "Flooring Adhesive", price: 1200 },
	{
		slug: "flooring-tapping-block",
		name: "Flooring Tapping Block",
		price: 500,
	},
	{ slug: "flooring-pull-bar", name: "Flooring Pull Bar", price: 600 },
	{ slug: "flooring-mallet", name: "Flooring Mallet", price: 800 },
	{ slug: "flooring-level", name: "Flooring Level", price: 900 },

	// Building a Deck or Patio
	{ slug: "deck-wood", name: "Deck Wood", price: 3500 },
	{ slug: "deck-screws", name: "Deck Screws", price: 1500 },
	{ slug: "deck-stain", name: "Deck Stain", price: 2500 },
	{ slug: "deck-sealer", name: "Deck Sealer", price: 2000 },
	{ slug: "deck-joist-hanger", name: "Deck Joist Hanger", price: 700 },
	{ slug: "deck-post-cap", name: "Deck Post Cap", price: 500 },
	{ slug: "deck-railing", name: "Deck Railing", price: 4000 },
	{ slug: "deck-flashing", name: "Deck Flashing", price: 1000 },
	{ slug: "deck-brush", name: "Deck Brush", price: 600 },
	{ slug: "deck-sander", name: "Deck Sander", price: 3000 },

	// Updating Light Fixtures
	{ slug: "light-fixture", name: "Light Fixture", price: 5000 },
	{ slug: "light-bulb", name: "Light Bulb", price: 300 },
	{ slug: "light-switch", name: "Light Switch", price: 400 },
	{ slug: "light-dimmer", name: "Light Dimmer", price: 1500 },
	{ slug: "light-ceiling-fan", name: "Ceiling Fan", price: 8000 },
	{ slug: "light-chandelier", name: "Chandelier", price: 12000 },
	{ slug: "light-wall-sconce", name: "Wall Sconce", price: 3500 },
	{ slug: "light-track-lighting", name: "Track Lighting", price: 7000 },
	{ slug: "light-recessed-light", name: "Recessed Light", price: 4000 },
	{ slug: "light-outdoor-fixture", name: "Outdoor Fixture", price: 6000 },

	// Bathroom Remodels
	{ slug: "bathroom-faucet", name: "Bathroom Faucet", price: 4500 },
	{ slug: "bathroom-vanity", name: "Bathroom Vanity", price: 15000 },
	{ slug: "bathroom-tile", name: "Bathroom Tile", price: 2000 },
	{ slug: "bathroom-showerhead", name: "Showerhead", price: 3000 },
	{ slug: "bathroom-tub", name: "Bathtub", price: 20000 },
	{ slug: "bathroom-toilet", name: "Toilet", price: 10000 },
	{ slug: "bathroom-mirror", name: "Bathroom Mirror", price: 5000 },
	{ slug: "bathroom-plumbing-tool", name: "Plumbing Tool", price: 2500 },
	{ slug: "bathroom-towel-bar", name: "Towel Bar", price: 1500 },
	{ slug: "bathroom-cabinet", name: "Bathroom Cabinet", price: 8000 },

	// Installing Shelving or Storage
	{ slug: "shelf-bracket", name: "Shelf Bracket", price: 800 },
	{ slug: "shelf-board", name: "Shelf Board", price: 1200 },
	{ slug: "shelf-wall-anchor", name: "Wall Anchor", price: 300 },
	{ slug: "shelf-peg", name: "Shelf Peg", price: 200 },
	{ slug: "shelf-organizer", name: "Shelf Organizer", price: 2500 },
	{ slug: "shelf-closet-system", name: "Closet System", price: 10000 },
	{ slug: "shelf-storage-bin", name: "Storage Bin", price: 1500 },
	{ slug: "shelf-wire-shelving", name: "Wire Shelving", price: 3000 },
	{ slug: "shelf-cube-storage", name: "Cube Storage", price: 4000 },
	{ slug: "shelf-cabinet", name: "Storage Cabinet", price: 7000 },

	// Gardening and Landscaping
	{ slug: "garden-soil", name: "Garden Soil", price: 500 },
	{ slug: "garden-mulch", name: "Mulch", price: 700 },
	{ slug: "garden-plant", name: "Plant", price: 1500 },
	{ slug: "garden-shovel", name: "Shovel", price: 2000 },
	{ slug: "garden-rake", name: "Rake", price: 1500 },
	{ slug: "garden-hose", name: "Garden Hose", price: 2500 },
	{ slug: "garden-gloves", name: "Garden Gloves", price: 800 },
	{ slug: "garden-wheelbarrow", name: "Wheelbarrow", price: 6000 },
	{ slug: "garden-fertilizer", name: "Fertilizer", price: 1200 },
	{ slug: "garden-pruner", name: "Pruner", price: 1800 },

	// Repairing Drywall
	{ slug: "drywall-sheet", name: "Drywall Sheet", price: 1000 },
	{ slug: "drywall-joint-compound", name: "Joint Compound", price: 800 },
	{ slug: "drywall-tape", name: "Drywall Tape", price: 300 },
	{ slug: "drywall-sander", name: "Drywall Sander", price: 2500 },
	{ slug: "drywall-screw", name: "Drywall Screw", price: 500 },
	{ slug: "drywall-corner-bead", name: "Corner Bead", price: 700 },
	{ slug: "drywall-knife", name: "Drywall Knife", price: 600 },
	{ slug: "drywall-saw", name: "Drywall Saw", price: 1500 },
	{ slug: "drywall-anchor", name: "Drywall Anchor", price: 400 },
	{ slug: "drywall-patch", name: "Drywall Patch", price: 900 },

	// Kitchen Upgrades
	{ slug: "kitchen-backsplash-tile", name: "Backsplash Tile", price: 2000 },
	{ slug: "kitchen-countertop", name: "Countertop", price: 15000 },
	{ slug: "kitchen-cabinet-hardware", name: "Cabinet Hardware", price: 1200 },
	{ slug: "kitchen-sink", name: "Kitchen Sink", price: 8000 },
	{ slug: "kitchen-faucet", name: "Kitchen Faucet", price: 5000 },
	{ slug: "kitchen-appliance", name: "Kitchen Appliance", price: 20000 },
	{ slug: "kitchen-island", name: "Kitchen Island", price: 25000 },
	{ slug: "kitchen-lighting", name: "Kitchen Lighting", price: 7000 },
	{ slug: "kitchen-cabinet", name: "Kitchen Cabinet", price: 20000 },
	{ slug: "kitchen-range-hood", name: "Range Hood", price: 10000 },

	// Building Furniture
	{ slug: "furniture-wood", name: "Furniture Wood", price: 3000 },
	{ slug: "furniture-screw", name: "Furniture Screw", price: 500 },
	{ slug: "furniture-drill", name: "Power Drill", price: 8000 },
	{ slug: "furniture-saw", name: "Saw", price: 6000 },
	{ slug: "furniture-sander", name: "Sander", price: 4000 },
	{ slug: "furniture-glue", name: "Wood Glue", price: 700 },
	{ slug: "furniture-clamp", name: "Clamp", price: 1500 },
	{ slug: "furniture-varnish", name: "Varnish", price: 2000 },
	{ slug: "furniture-hinge", name: "Hinge", price: 300 },
	{ slug: "furniture-knob", name: "Knob", price: 400 },
];
