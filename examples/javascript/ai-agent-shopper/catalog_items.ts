import type { CatalogItem } from "./catalog";

const CATALOG_ITEMS: CatalogItem[] = [
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
	{
		slug: "flooring-underlayment",
		name: "Flooring Underlayment",
		price: 1000,
	},
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

	// Paint Colors (expanding the existing paint section)
	{ slug: "paint-beige", name: "Beige Paint", price: 2500 },
	{ slug: "paint-gray", name: "Gray Paint", price: 2500 },
	{ slug: "paint-yellow", name: "Yellow Paint", price: 2600 },
	{ slug: "paint-red", name: "Red Paint", price: 2600 },
	{ slug: "paint-green", name: "Green Paint", price: 2600 },
	{ slug: "paint-brown", name: "Brown Paint", price: 2500 },
	{ slug: "paint-black", name: "Black Paint", price: 2500 },
	{ slug: "paint-navy", name: "Navy Paint", price: 2600 },
	{ slug: "paint-purple", name: "Purple Paint", price: 2600 },
	{ slug: "paint-pink", name: "Pink Paint", price: 2600 },
	{ slug: "paint-orange", name: "Orange Paint", price: 2600 },
	{ slug: "paint-teal", name: "Teal Paint", price: 2600 },
	{ slug: "paint-cream", name: "Cream Paint", price: 2500 },
	{ slug: "paint-sage", name: "Sage Paint", price: 2600 },
	{ slug: "paint-maroon", name: "Maroon Paint", price: 2600 },

	// Paint Finishes
	{ slug: "paint-matte", name: "Matte Paint Finish", price: 2800 },
	{ slug: "paint-eggshell", name: "Eggshell Paint Finish", price: 2800 },
	{ slug: "paint-satin", name: "Satin Paint Finish", price: 3000 },
	{ slug: "paint-semi-gloss", name: "Semi-Gloss Paint Finish", price: 3200 },
	{ slug: "paint-gloss", name: "Gloss Paint Finish", price: 3400 },

	// Additional Painting Supplies
	{ slug: "paint-sprayer", name: "Paint Sprayer", price: 15000 },
	{ slug: "paint-ladder", name: "Painting Ladder", price: 8000 },
	{ slug: "paint-respirator", name: "Paint Respirator", price: 3000 },
	{ slug: "paint-goggles", name: "Safety Goggles", price: 1500 },
	{ slug: "paint-extension-pole", name: "Extension Pole", price: 2500 },
	{ slug: "paint-bucket", name: "Paint Bucket", price: 800 },
	{ slug: "paint-strainer", name: "Paint Strainer", price: 400 },
	{
		slug: "paint-mixer-attachment",
		name: "Paint Mixer Attachment",
		price: 1200,
	},

	// Power Tools
	{ slug: "power-circular-saw", name: "Circular Saw", price: 12000 },
	{ slug: "power-jigsaw", name: "Jigsaw", price: 9000 },
	{
		slug: "power-reciprocating-saw",
		name: "Reciprocating Saw",
		price: 11000,
	},
	{ slug: "power-angle-grinder", name: "Angle Grinder", price: 8000 },
	{ slug: "power-rotary-tool", name: "Rotary Tool", price: 7000 },
	{ slug: "power-heat-gun", name: "Heat Gun", price: 5000 },
	{ slug: "power-router", name: "Router", price: 13000 },
	{ slug: "power-planer", name: "Power Planer", price: 10000 },
	{ slug: "power-oscillating-tool", name: "Oscillating Tool", price: 8000 },
	{ slug: "power-impact-driver", name: "Impact Driver", price: 9000 },

	// Hand Tools
	{ slug: "hand-hammer-claw", name: "Claw Hammer", price: 2500 },
	{ slug: "hand-hammer-ball-peen", name: "Ball Peen Hammer", price: 2800 },
	{ slug: "hand-wrench-adjustable", name: "Adjustable Wrench", price: 2000 },
	{ slug: "hand-pliers-needlenose", name: "Needlenose Pliers", price: 1800 },
	{
		slug: "hand-pliers-channel-lock",
		name: "Channel Lock Pliers",
		price: 2200,
	},
	{ slug: "hand-screwdriver-set", name: "Screwdriver Set", price: 3500 },
	{ slug: "hand-chisel-set", name: "Wood Chisel Set", price: 4000 },
	{ slug: "hand-level-torpedo", name: "Torpedo Level", price: 1500 },
	{ slug: "hand-utility-knife", name: "Utility Knife", price: 1000 },
	{ slug: "hand-putty-knife", name: "Putty Knife", price: 800 },

	// Measuring and Layout Tools
	{ slug: "measure-tape-25ft", name: "25ft Tape Measure", price: 1500 },
	{ slug: "measure-tape-100ft", name: "100ft Tape Measure", price: 2500 },
	{
		slug: "measure-laser-distance",
		name: "Laser Distance Measure",
		price: 8000,
	},
	{ slug: "measure-stud-finder", name: "Stud Finder", price: 2500 },
	{ slug: "measure-angle-finder", name: "Digital Angle Finder", price: 3500 },
	{ slug: "measure-chalk-line", name: "Chalk Line", price: 1000 },
	{ slug: "measure-carpenter-pencil", name: "Carpenter Pencil", price: 200 },
	{
		slug: "measure-combination-square",
		name: "Combination Square",
		price: 2000,
	},

	// Fasteners and Hardware
	{
		slug: "fastener-wood-screw-assorted",
		name: "Wood Screw Assortment",
		price: 2000,
	},
	{
		slug: "fastener-drywall-screw-assorted",
		name: "Drywall Screw Assortment",
		price: 1800,
	},
	{
		slug: "fastener-deck-screw-assorted",
		name: "Deck Screw Assortment",
		price: 2500,
	},
	{ slug: "fastener-nail-assorted", name: "Nail Assortment", price: 1500 },
	{
		slug: "fastener-anchor-assorted",
		name: "Wall Anchor Assortment",
		price: 1200,
	},
	{ slug: "fastener-bolt-assorted", name: "Bolt Assortment", price: 2200 },
	{ slug: "fastener-nut-assorted", name: "Nut Assortment", price: 1000 },
	{ slug: "fastener-washer-assorted", name: "Washer Assortment", price: 800 },

	// Plumbing
	{ slug: "plumbing-pipe-pvc", name: "PVC Pipe", price: 1500 },
	{ slug: "plumbing-pipe-copper", name: "Copper Pipe", price: 3000 },
	{ slug: "plumbing-fitting-pvc", name: "PVC Fittings", price: 500 },
	{ slug: "plumbing-fitting-copper", name: "Copper Fittings", price: 1000 },
	{ slug: "plumbing-pipe-wrench", name: "Pipe Wrench", price: 3500 },
	{ slug: "plumbing-snake", name: "Plumbing Snake", price: 4000 },
	{
		slug: "plumbing-compression-fitting",
		name: "Compression Fittings",
		price: 800,
	},
	{ slug: "plumbing-pipe-insulation", name: "Pipe Insulation", price: 1000 },

	// Electrical
	{ slug: "electrical-wire-14-2", name: "14-2 Electrical Wire", price: 6000 },
	{ slug: "electrical-wire-12-2", name: "12-2 Electrical Wire", price: 7000 },
	{ slug: "electrical-outlet", name: "Electrical Outlet", price: 500 },
	{ slug: "electrical-outlet-cover", name: "Outlet Cover", price: 200 },
	{ slug: "electrical-wire-stripper", name: "Wire Stripper", price: 2000 },
	{ slug: "electrical-voltage-tester", name: "Voltage Tester", price: 1500 },
	{ slug: "electrical-junction-box", name: "Junction Box", price: 800 },
	{ slug: "electrical-conduit", name: "Electrical Conduit", price: 1500 },

	// Safety Equipment
	{ slug: "safety-glasses", name: "Safety Glasses", price: 1500 },
	{ slug: "safety-gloves-leather", name: "Leather Work Gloves", price: 2000 },
	{ slug: "safety-gloves-nitrile", name: "Nitrile Gloves", price: 1500 },
	{ slug: "safety-ear-protection", name: "Ear Protection", price: 2500 },
	{ slug: "safety-dust-mask", name: "Dust Mask", price: 1000 },
	{ slug: "safety-knee-pads", name: "Knee Pads", price: 2500 },
	{ slug: "safety-first-aid", name: "First Aid Kit", price: 3000 },
	{
		slug: "safety-fire-extinguisher",
		name: "Fire Extinguisher",
		price: 5000,
	},

	// HVAC
	{ slug: "hvac-air-filter", name: "Air Filter", price: 2000 },
	{ slug: "hvac-duct-tape", name: "HVAC Duct Tape", price: 1500 },
	{ slug: "hvac-thermostat", name: "Programmable Thermostat", price: 8000 },
	{ slug: "hvac-vent-cover", name: "Vent Cover", price: 1200 },
	{ slug: "hvac-insulation", name: "HVAC Insulation", price: 3000 },
	{ slug: "hvac-refrigerant", name: "AC Refrigerant", price: 5000 },

	// Windows and Doors
	{ slug: "door-knob-brass", name: "Brass Door Knob", price: 3000 },
	{ slug: "door-knob-nickel", name: "Nickel Door Knob", price: 3000 },
	{ slug: "door-knob-bronze", name: "Bronze Door Knob", price: 3000 },
	{ slug: "door-hinge-brass", name: "Brass Door Hinge", price: 1000 },
	{ slug: "door-hinge-nickel", name: "Nickel Door Hinge", price: 1000 },
	{ slug: "door-hinge-bronze", name: "Bronze Door Hinge", price: 1000 },
	{ slug: "window-lock", name: "Window Lock", price: 1500 },
	{ slug: "door-sweep", name: "Door Sweep", price: 1200 },
	{ slug: "door-weatherstrip", name: "Door Weatherstripping", price: 1500 },
	{
		slug: "window-weatherstrip",
		name: "Window Weatherstripping",
		price: 1500,
	},

	// Outdoor and Garden Additional Items
	{ slug: "outdoor-landscape-fabric", name: "Landscape Fabric", price: 2000 },
	{ slug: "outdoor-edging", name: "Garden Edging", price: 1500 },
	{ slug: "outdoor-sprinkler", name: "Garden Sprinkler", price: 2500 },
	{ slug: "outdoor-timer", name: "Water Timer", price: 3000 },
	{ slug: "outdoor-trellis", name: "Garden Trellis", price: 4000 },
	{ slug: "outdoor-spray-nozzle", name: "Spray Nozzle", price: 1500 },
	{ slug: "outdoor-plant-stakes", name: "Plant Stakes", price: 800 },
	{ slug: "outdoor-potting-soil", name: "Potting Soil", price: 1000 },

	// Cabinet Hardware Colors
	{ slug: "cabinet-pull-brass", name: "Brass Cabinet Pull", price: 800 },
	{ slug: "cabinet-pull-nickel", name: "Nickel Cabinet Pull", price: 800 },
	{ slug: "cabinet-pull-bronze", name: "Bronze Cabinet Pull", price: 800 },
	{ slug: "cabinet-pull-black", name: "Black Cabinet Pull", price: 800 },
	{ slug: "cabinet-pull-copper", name: "Copper Cabinet Pull", price: 800 },
	{ slug: "cabinet-knob-brass", name: "Brass Cabinet Knob", price: 600 },
	{ slug: "cabinet-knob-nickel", name: "Nickel Cabinet Knob", price: 600 },
	{ slug: "cabinet-knob-bronze", name: "Bronze Cabinet Knob", price: 600 },
	{ slug: "cabinet-knob-black", name: "Black Cabinet Knob", price: 600 },
	{ slug: "cabinet-knob-copper", name: "Copper Cabinet Knob", price: 600 },

	// Lumber and Building Materials
	{ slug: "lumber-2x4-8ft", name: "2x4 Lumber 8ft", price: 800 },
	{ slug: "lumber-2x4-10ft", name: "2x4 Lumber 10ft", price: 1000 },
	{ slug: "lumber-2x4-12ft", name: "2x4 Lumber 12ft", price: 1200 },
	{ slug: "lumber-2x6-8ft", name: "2x6 Lumber 8ft", price: 1200 },
	{ slug: "lumber-2x6-10ft", name: "2x6 Lumber 10ft", price: 1500 },
	{ slug: "lumber-2x6-12ft", name: "2x6 Lumber 12ft", price: 1800 },
	{ slug: "lumber-4x4-8ft", name: "4x4 Lumber 8ft", price: 2000 },
	{ slug: "lumber-4x4-10ft", name: "4x4 Lumber 10ft", price: 2500 },
	{ slug: "lumber-4x4-12ft", name: "4x4 Lumber 12ft", price: 3000 },
	{ slug: "plywood-1/2-4x8", name: '1/2" Plywood 4x8', price: 3500 },
	{ slug: "plywood-3/4-4x8", name: '3/4" Plywood 4x8', price: 4500 },
	{ slug: "osb-7/16-4x8", name: '7/16" OSB 4x8', price: 2500 },
	{ slug: "mdf-3/4-4x8", name: '3/4" MDF 4x8', price: 3000 },

	// More Paint Colors (Specialty)
	{ slug: "paint-metallic-gold", name: "Metallic Gold Paint", price: 3500 },
	{
		slug: "paint-metallic-silver",
		name: "Metallic Silver Paint",
		price: 3500,
	},
	{
		slug: "paint-metallic-copper",
		name: "Metallic Copper Paint",
		price: 3500,
	},
	{
		slug: "paint-metallic-bronze",
		name: "Metallic Bronze Paint",
		price: 3500,
	},
	{ slug: "paint-glow-white", name: "Glow-in-Dark White Paint", price: 4000 },
	{ slug: "paint-glow-green", name: "Glow-in-Dark Green Paint", price: 4000 },
	{ slug: "paint-glow-blue", name: "Glow-in-Dark Blue Paint", price: 4000 },
	{ slug: "paint-chalkboard", name: "Chalkboard Paint", price: 3000 },
	{ slug: "paint-magnetic", name: "Magnetic Paint", price: 3500 },
	{ slug: "paint-textured-sand", name: "Textured Sand Paint", price: 3000 },

	// Concrete and Masonry
	{ slug: "concrete-mix-60lb", name: "Concrete Mix 60lb", price: 800 },
	{ slug: "concrete-mix-80lb", name: "Concrete Mix 80lb", price: 1000 },
	{ slug: "mortar-mix-60lb", name: "Mortar Mix 60lb", price: 900 },
	{ slug: "concrete-block-8in", name: '8" Concrete Block', price: 300 },
	{ slug: "concrete-block-6in", name: '6" Concrete Block', price: 250 },
	{ slug: "brick-red", name: "Red Brick", price: 100 },
	{ slug: "brick-brown", name: "Brown Brick", price: 100 },
	{ slug: "brick-gray", name: "Gray Brick", price: 100 },
	{ slug: "concrete-sealer-1gal", name: "Concrete Sealer 1gal", price: 3000 },
	{ slug: "concrete-color-brown", name: "Brown Concrete Color", price: 1500 },
	{ slug: "concrete-color-gray", name: "Gray Concrete Color", price: 1500 },
	{
		slug: "concrete-color-terra",
		name: "Terra Cotta Concrete Color",
		price: 1500,
	},

	// More Power Tool Accessories
	{ slug: "drill-bit-wood-set", name: "Wood Drill Bit Set", price: 3000 },
	{
		slug: "drill-bit-masonry-set",
		name: "Masonry Drill Bit Set",
		price: 3500,
	},
	{ slug: "drill-bit-metal-set", name: "Metal Drill Bit Set", price: 4000 },
	{ slug: "saw-blade-7-1/4", name: '7-1/4" Circular Saw Blade', price: 2500 },
	{ slug: "saw-blade-10", name: '10" Table Saw Blade', price: 4000 },
	{ slug: "saw-blade-12", name: '12" Miter Saw Blade', price: 5000 },
	{ slug: "router-bit-set", name: "Router Bit Set", price: 8000 },
	{
		slug: "sanding-disc-5in-60",
		name: '5" 60-Grit Sanding Disc',
		price: 500,
	},
	{
		slug: "sanding-disc-5in-120",
		name: '5" 120-Grit Sanding Disc',
		price: 500,
	},
	{
		slug: "sanding-disc-5in-220",
		name: '5" 220-Grit Sanding Disc',
		price: 500,
	},

	// Additional Tool Storage
	{ slug: "toolbox-plastic-22in", name: '22" Plastic Toolbox', price: 3000 },
	{ slug: "toolbox-metal-26in", name: '26" Metal Toolbox', price: 5000 },
	{ slug: "tool-chest-5-drawer", name: "5-Drawer Tool Chest", price: 15000 },
	{
		slug: "tool-cabinet-rolling",
		name: "Rolling Tool Cabinet",
		price: 25000,
	},
	{ slug: "tool-bag-18in", name: '18" Tool Bag', price: 4000 },
	{ slug: "tool-belt-leather", name: "Leather Tool Belt", price: 6000 },
	{ slug: "tool-pouch-nail", name: "Nail Pouch", price: 2000 },
	{
		slug: "tool-bucket-organizer",
		name: "Bucket Tool Organizer",
		price: 2500,
	},

	// Roofing Materials
	{ slug: "shingle-black", name: "Black Roof Shingles", price: 3000 },
	{ slug: "shingle-brown", name: "Brown Roof Shingles", price: 3000 },
	{ slug: "shingle-gray", name: "Gray Roof Shingles", price: 3000 },
	{ slug: "shingle-green", name: "Green Roof Shingles", price: 3000 },
	{ slug: "roofing-felt-15lb", name: "15lb Roofing Felt", price: 2000 },
	{ slug: "roofing-felt-30lb", name: "30lb Roofing Felt", price: 2500 },
	{ slug: "roof-vent", name: "Roof Vent", price: 1500 },
	{ slug: "roof-flashing", name: "Roof Flashing", price: 1000 },
	{ slug: "gutter-white-10ft", name: "10ft White Gutter", price: 1500 },
	{ slug: "gutter-brown-10ft", name: "10ft Brown Gutter", price: 1500 },

	// More Plumbing Supplies
	{ slug: "pipe-pex-1/2in-100ft", name: '1/2" PEX Pipe 100ft', price: 5000 },
	{ slug: "pipe-pex-3/4in-100ft", name: '3/4" PEX Pipe 100ft', price: 7000 },
	{ slug: "pipe-cpvc-1/2in-10ft", name: '1/2" CPVC Pipe 10ft', price: 1000 },
	{ slug: "pipe-cpvc-3/4in-10ft", name: '3/4" CPVC Pipe 10ft', price: 1500 },
	{
		slug: "pipe-black-1/2in-10ft",
		name: '1/2" Black Pipe 10ft',
		price: 2000,
	},
	{
		slug: "pipe-black-3/4in-10ft",
		name: '3/4" Black Pipe 10ft',
		price: 2500,
	},
	{
		slug: "water-filter-whole",
		name: "Whole House Water Filter",
		price: 15000,
	},
	{ slug: "water-softener", name: "Water Softener", price: 30000 },
	{ slug: "water-heater-40gal", name: "40gal Water Heater", price: 45000 },
	{ slug: "water-heater-50gal", name: "50gal Water Heater", price: 55000 },

	// More Electrical Supplies
	{ slug: "wire-14-3", name: "14-3 Electrical Wire", price: 7000 },
	{ slug: "wire-12-3", name: "12-3 Electrical Wire", price: 8000 },
	{ slug: "wire-10-2", name: "10-2 Electrical Wire", price: 9000 },
	{ slug: "wire-connector-red", name: "Red Wire Connectors", price: 500 },
	{
		slug: "wire-connector-yellow",
		name: "Yellow Wire Connectors",
		price: 500,
	},
	{ slug: "wire-connector-blue", name: "Blue Wire Connectors", price: 500 },
	{
		slug: "electrical-box-old-work",
		name: "Old Work Electrical Box",
		price: 400,
	},
	{
		slug: "electrical-box-new-work",
		name: "New Work Electrical Box",
		price: 300,
	},
	{ slug: "conduit-1/2in-10ft", name: '1/2" Conduit 10ft', price: 800 },
	{ slug: "conduit-3/4in-10ft", name: '3/4" Conduit 10ft', price: 1000 },

	// Outdoor Living
	{ slug: "paver-gray", name: "Gray Paver", price: 300 },
	{ slug: "paver-red", name: "Red Paver", price: 300 },
	{ slug: "paver-tan", name: "Tan Paver", price: 300 },
	{ slug: "retaining-wall-block", name: "Retaining Wall Block", price: 500 },
	{
		slug: "landscape-stone-white",
		name: "White Landscape Stone",
		price: 400,
	},
	{ slug: "landscape-stone-red", name: "Red Landscape Stone", price: 400 },
	{
		slug: "landscape-stone-black",
		name: "Black Landscape Stone",
		price: 400,
	},
	{ slug: "landscape-timber", name: "Landscape Timber", price: 800 },
	{ slug: "outdoor-fire-pit", name: "Fire Pit Kit", price: 15000 },
	{ slug: "patio-umbrella", name: "Patio Umbrella", price: 10000 },

	// Lawn Care Equipment
	{ slug: "mower-push", name: "Push Lawn Mower", price: 25000 },
	{
		slug: "mower-self-propelled",
		name: "Self-Propelled Mower",
		price: 35000,
	},
	{ slug: "trimmer-gas", name: "Gas String Trimmer", price: 15000 },
	{ slug: "trimmer-battery", name: "Battery String Trimmer", price: 20000 },
	{ slug: "blower-gas", name: "Gas Leaf Blower", price: 15000 },
	{ slug: "blower-battery", name: "Battery Leaf Blower", price: 20000 },
	{ slug: "edger-gas", name: "Gas Lawn Edger", price: 20000 },
	{ slug: "sprayer-2gal", name: "2gal Garden Sprayer", price: 3000 },
	{ slug: "spreader-broadcast", name: "Broadcast Spreader", price: 5000 },
	{ slug: "spreader-drop", name: "Drop Spreader", price: 4000 },

	// Cleaning Supplies
	{ slug: "vacuum-wet-dry", name: "Wet/Dry Vacuum", price: 10000 },
	{ slug: "pressure-washer", name: "Pressure Washer", price: 25000 },
	{ slug: "broom-push", name: "Push Broom", price: 2000 },
	{ slug: "mop-commercial", name: "Commercial Mop", price: 2500 },
	{ slug: "bucket-5gal", name: "5gal Bucket", price: 500 },
	{ slug: "cleaner-concrete", name: "Concrete Cleaner", price: 2000 },
	{ slug: "cleaner-wood", name: "Wood Cleaner", price: 1500 },
	{ slug: "degreaser", name: "Heavy Duty Degreaser", price: 1800 },

	// Workshop Equipment
	{ slug: "workbench-6ft", name: "6ft Workbench", price: 20000 },
	{ slug: "vise-6in", name: '6" Bench Vise', price: 8000 },
	{ slug: "air-compressor-6gal", name: "6gal Air Compressor", price: 15000 },
	{ slug: "air-hose-50ft", name: "50ft Air Hose", price: 3000 },
	{ slug: "shop-fan-24in", name: '24" Shop Fan', price: 8000 },
	{ slug: "shop-vac-12gal", name: "12gal Shop Vacuum", price: 10000 },
	{ slug: "work-light-led", name: "LED Work Light", price: 5000 },
	{ slug: "extension-cord-50ft", name: "50ft Extension Cord", price: 3000 },

	// More Safety Equipment
	{ slug: "hard-hat-white", name: "White Hard Hat", price: 2000 },
	{ slug: "hard-hat-yellow", name: "Yellow Hard Hat", price: 2000 },
	{ slug: "safety-vest-l", name: "Large Safety Vest", price: 1500 },
	{ slug: "safety-vest-xl", name: "XL Safety Vest", price: 1500 },
	{ slug: "work-boots-9", name: "Size 9 Work Boots", price: 8000 },
	{ slug: "work-boots-10", name: "Size 10 Work Boots", price: 8000 },
	{ slug: "work-boots-11", name: "Size 11 Work Boots", price: 8000 },
	{ slug: "work-boots-12", name: "Size 12 Work Boots", price: 8000 },
	{ slug: "work-boots-13", name: "Size 13 Work Boots", price: 8000 },
	{
		slug: "safety-glasses-tinted",
		name: "Tinted Safety Glasses",
		price: 2000,
	},
	{ slug: "safety-glasses-clear", name: "Clear Safety Glasses", price: 1500 },
	{ slug: "face-shield", name: "Full Face Shield", price: 3000 },
	{ slug: "respirator-half", name: "Half-Face Respirator", price: 4000 },
	{ slug: "respirator-full", name: "Full-Face Respirator", price: 8000 },
	{
		slug: "respirator-filter-p100",
		name: "P100 Respirator Filters",
		price: 2000,
	},
	{
		slug: "respirator-filter-organic",
		name: "Organic Vapor Filters",
		price: 2500,
	},

	// Insulation Materials
	{
		slug: "insulation-r13-kraft",
		name: "R13 Kraft Faced Insulation",
		price: 4000,
	},
	{
		slug: "insulation-r19-kraft",
		name: "R19 Kraft Faced Insulation",
		price: 5000,
	},
	{
		slug: "insulation-r30-unfaced",
		name: "R30 Unfaced Insulation",
		price: 6000,
	},
	{
		slug: "insulation-foam-board-1in",
		name: '1" Foam Board Insulation',
		price: 2500,
	},
	{
		slug: "insulation-foam-board-2in",
		name: '2" Foam Board Insulation',
		price: 4000,
	},
	{
		slug: "insulation-spray-foam",
		name: "Spray Foam Insulation Kit",
		price: 8000,
	},
	{ slug: "insulation-pipe-wrap", name: "Pipe Insulation Wrap", price: 500 },
	{ slug: "insulation-tape", name: "Insulation Tape", price: 800 },
	{
		slug: "weatherstrip-door-bottom",
		name: "Door Bottom Weatherstrip",
		price: 1200,
	},
	{
		slug: "weatherstrip-window-v",
		name: "V-Seal Window Weatherstrip",
		price: 1000,
	},

	// More Plumbing Fixtures
	{ slug: "toilet-white-round", name: "White Round Toilet", price: 15000 },
	{
		slug: "toilet-white-elongated",
		name: "White Elongated Toilet",
		price: 18000,
	},
	{ slug: "toilet-beige-round", name: "Beige Round Toilet", price: 15000 },
	{
		slug: "toilet-beige-elongated",
		name: "Beige Elongated Toilet",
		price: 18000,
	},
	{ slug: "sink-bathroom-white", name: "White Bathroom Sink", price: 8000 },
	{ slug: "sink-bathroom-beige", name: "Beige Bathroom Sink", price: 8000 },
	{
		slug: "sink-kitchen-stainless",
		name: "Stainless Kitchen Sink",
		price: 12000,
	},
	{ slug: "sink-kitchen-white", name: "White Kitchen Sink", price: 10000 },
	{
		slug: "faucet-bathroom-chrome",
		name: "Chrome Bathroom Faucet",
		price: 5000,
	},
	{
		slug: "faucet-bathroom-bronze",
		name: "Bronze Bathroom Faucet",
		price: 6000,
	},
	{
		slug: "faucet-kitchen-chrome",
		name: "Chrome Kitchen Faucet",
		price: 8000,
	},
	{
		slug: "faucet-kitchen-bronze",
		name: "Bronze Kitchen Faucet",
		price: 9000,
	},

	// More Power Tools
	{ slug: "table-saw-10in", name: '10" Table Saw', price: 50000 },
	{ slug: "miter-saw-10in", name: '10" Miter Saw', price: 30000 },
	{ slug: "miter-saw-12in", name: '12" Miter Saw', price: 40000 },
	{ slug: "band-saw-9in", name: '9" Band Saw', price: 35000 },
	{ slug: "band-saw-14in", name: '14" Band Saw', price: 45000 },
	{ slug: "drill-press-10in", name: '10" Drill Press', price: 25000 },
	{ slug: "drill-press-12in", name: '12" Drill Press', price: 35000 },
	{ slug: "air-nailer-brad", name: "Brad Nailer", price: 12000 },
	{ slug: "air-nailer-finish", name: "Finish Nailer", price: 15000 },
	{ slug: "air-nailer-framing", name: "Framing Nailer", price: 25000 },

	// More Fasteners
	{ slug: "nail-brad-1in", name: '1" Brad Nails', price: 500 },
	{ slug: "nail-brad-1-1/4in", name: '1-1/4" Brad Nails', price: 600 },
	{ slug: "nail-brad-1-1/2in", name: '1-1/2" Brad Nails', price: 700 },
	{ slug: "nail-finish-2in", name: '2" Finish Nails', price: 800 },
	{ slug: "nail-finish-2-1/2in", name: '2-1/2" Finish Nails', price: 900 },
	{ slug: "nail-finish-3in", name: '3" Finish Nails', price: 1000 },
	{ slug: "nail-framing-3in", name: '3" Framing Nails', price: 1200 },
	{ slug: "nail-framing-3-1/2in", name: '3-1/2" Framing Nails', price: 1300 },
	{
		slug: "screw-deck-2-1/2in-tan",
		name: '2-1/2" Tan Deck Screws',
		price: 1500,
	},
	{
		slug: "screw-deck-2-1/2in-brown",
		name: '2-1/2" Brown Deck Screws',
		price: 1500,
	},

	// More Garden Supplies
	{ slug: "plant-rose-red", name: "Red Rose Bush", price: 2500 },
	{ slug: "plant-rose-pink", name: "Pink Rose Bush", price: 2500 },
	{ slug: "plant-rose-yellow", name: "Yellow Rose Bush", price: 2500 },
	{ slug: "plant-hydrangea-blue", name: "Blue Hydrangea", price: 3000 },
	{ slug: "plant-hydrangea-pink", name: "Pink Hydrangea", price: 3000 },
	{ slug: "plant-evergreen-small", name: "Small Evergreen", price: 4000 },
	{ slug: "plant-evergreen-medium", name: "Medium Evergreen", price: 6000 },
	{ slug: "plant-fruit-apple", name: "Apple Tree", price: 8000 },
	{ slug: "plant-fruit-peach", name: "Peach Tree", price: 8000 },
	{ slug: "plant-fruit-cherry", name: "Cherry Tree", price: 8000 },

	// More Outdoor Power Equipment
	{ slug: "chainsaw-16in-gas", name: '16" Gas Chainsaw', price: 25000 },
	{ slug: "chainsaw-18in-gas", name: '18" Gas Chainsaw', price: 30000 },
	{
		slug: "chainsaw-16in-battery",
		name: '16" Battery Chainsaw',
		price: 35000,
	},
	{ slug: "hedge-trimmer-gas", name: "Gas Hedge Trimmer", price: 20000 },
	{
		slug: "hedge-trimmer-battery",
		name: "Battery Hedge Trimmer",
		price: 25000,
	},
	{ slug: "tiller-front-tine", name: "Front Tine Tiller", price: 40000 },
	{ slug: "tiller-rear-tine", name: "Rear Tine Tiller", price: 60000 },
	{ slug: "aerator-push", name: "Push Aerator", price: 15000 },
	{ slug: "aerator-tow", name: "Tow-Behind Aerator", price: 25000 },
	{ slug: "dethatcher-electric", name: "Electric Dethatcher", price: 20000 },

	// Storage Solutions
	{
		slug: "shelf-garage-5tier",
		name: "5-Tier Garage Shelving",
		price: 12000,
	},
	{ slug: "cabinet-garage-wall", name: "Garage Wall Cabinet", price: 15000 },
	{ slug: "cabinet-garage-base", name: "Garage Base Cabinet", price: 20000 },
	{ slug: "storage-hook-bike", name: "Bike Storage Hook", price: 1500 },
	{ slug: "storage-hook-ladder", name: "Ladder Storage Hook", price: 2000 },
	{ slug: "storage-rack-lumber", name: "Lumber Storage Rack", price: 8000 },
	{ slug: "storage-bin-tote", name: "Storage Tote Bin", price: 1500 },
	{ slug: "storage-bin-wheeled", name: "Wheeled Storage Bin", price: 2500 },
	{ slug: "pegboard-white", name: "White Pegboard", price: 2000 },
	{ slug: "pegboard-brown", name: "Brown Pegboard", price: 2000 },

	// Smart Home Products
	{ slug: "smart-thermostat", name: "Smart Thermostat", price: 25000 },
	{ slug: "smart-doorbell", name: "Smart Doorbell", price: 20000 },
	{ slug: "smart-lock-deadbolt", name: "Smart Deadbolt", price: 15000 },
	{ slug: "smart-garage-opener", name: "Smart Garage Opener", price: 30000 },
	{ slug: "smart-light-switch", name: "Smart Light Switch", price: 4000 },
	{ slug: "smart-outlet", name: "Smart Outlet", price: 3000 },
	{ slug: "smart-flood-sensor", name: "Smart Flood Sensor", price: 5000 },
	{ slug: "smart-smoke-detector", name: "Smart Smoke Detector", price: 8000 },
	{
		slug: "smart-camera-outdoor",
		name: "Outdoor Smart Camera",
		price: 15000,
	},
	{ slug: "smart-camera-indoor", name: "Indoor Smart Camera", price: 12000 },

	// Ventilation Equipment
	{ slug: "fan-bathroom-50cfm", name: "50 CFM Bathroom Fan", price: 5000 },
	{ slug: "fan-bathroom-80cfm", name: "80 CFM Bathroom Fan", price: 7000 },
	{ slug: "fan-bathroom-110cfm", name: "110 CFM Bathroom Fan", price: 9000 },
	{ slug: "fan-attic-powered", name: "Powered Attic Fan", price: 15000 },
	{ slug: "vent-roof-static", name: "Static Roof Vent", price: 3000 },
	{ slug: "vent-soffit-white", name: "White Soffit Vent", price: 500 },
	{ slug: "vent-soffit-brown", name: "Brown Soffit Vent", price: 500 },
	{ slug: "vent-gable-white", name: "White Gable Vent", price: 2500 },
	{ slug: "vent-gable-brown", name: "Brown Gable Vent", price: 2500 },
	{ slug: "duct-flexible-6in", name: '6" Flexible Duct', price: 2000 },
];

export default CATALOG_ITEMS;
