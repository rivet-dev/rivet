import {
	faCloud,
	faComputer,
	faServer,
	Icon,
	type IconProp,
} from "@rivet-gg/icons";
import { AssetImage } from "../asset-image";
import { convertEmojiToUriFriendlyString } from "../lib/emoji";

export const REGION_ICON: Record<string, string | IconProp> = {
	default: faServer,
	unknown: "❓",
	atlanta: "🇺🇸", // Atlanta
	san_francisco: "🇺🇸", // San Francisco
	frankfurt: "🇩🇪", // Frankfurt
	sydney: "🇦🇺", // Sydney
	tokyo: "🇯🇵", // Tokyo
	mumbai: "🇮🇳", // Mumbai
	toronto: "🇨🇦", // Toronto
	washington_dc: "🇺🇸", // Washington DC
	dallas: "🇺🇸", // Dallas
	new_york_city: "🇺🇸", // Newark
	london: "🇬🇧", // London
	singapore: "🇸🇬", // Singapore
	amsterdam: "🇳🇱", // Amsterdam
	chicago: "🇺🇸", // Chicago
	bangalore: "🇮🇳", // Bangalore
	paris: "🇫🇷", // Paris
	seattle: "🇺🇸", // Seattle
	stockholm: "🇸🇪", // Stockholm
	newark: "🇺🇸", // Newark
	sao_paulo: "🇧🇷", // Sao Paulo
	chennai: "🇮🇳", // Chennai
	osaka: "🇯🇵", // Osaka
	milan: "🇮🇹", // Milan
	miami: "🇺🇸", // Miami
	jakarta: "🇮🇩", // Jakarta
	los_angeles: "🇺🇸", // Los Angeles
	atl: "🇺🇸", // Atlanta
	sfo: "🇺🇸", // San Francisco
	fra: "🇩🇪", // Frankfurt
	syd: "🇦🇺", // Sydney
	tok: "🇯🇵", // Tokyo
	mba: "🇮🇳", // Mumbai
	tor: "🇨🇦", // Toronto
	dca: "🇺🇸", // Washington DC
	dfw: "🇺🇸", // Dallas
	ewr: "🇺🇸", // Newark
	lon: "🇬🇧", // London
	sgp: "🇸🇬", // Singapore
	lax: "🇺🇸", // Los Angeles
	osa: "🇯🇵", // Osaka
	gru: "🇧🇷", // Sao Paulo
	bom: "🇮🇳", // Mumbai
	sin: "🇸🇬", // Singapore
	"eu-central-1": "🇩🇪", // Frankfurt
	"us-east-1": "🇺🇸", // Northern Virginia
	"us-west-1": "🇺🇸", // Oregon
	"ap-southeast-1": "🇸🇬", // Singapore
	cloud: faCloud,
};

export const REGION_LABEL: Record<string, string> = {
	default: "Default",
	unknown: "Unknown",
	atlanta: "Atlanta, Georgia, USA",
	san_francisco: "San Francisco",
	frankfurt: "Frankfurt",
	sydney: "Sydney",
	tokyo: "Tokyo",
	mumbai: "Mumbai",
	toronto: "Toronto",
	washington_dc: "Washington DC",
	dallas: "Dallas",
	new_york_city: "New York City",
	london: "London",
	singapore: "Singapore",
	amsterdam: "Amsterdam",
	chicago: "Chicago",
	bangalore: "Bangalore",
	paris: "Paris",
	seattle: "Seattle",
	stockholm: "Stockholm",
	newark: "Newark",
	sao_paulo: "Sao Paulo",
	chennai: "Chennai",
	osaka: "Osaka",
	milan: "Milan",
	miami: "Miami",
	jakarta: "Jakarta",
	los_angeles: "Los Angeles",
	atl: "Atlanta, Georgia, USA",
	sfo: "San Francisco, California, USA",
	fra: "Frankfurt, Germany",
	syd: "Sydney, Australia",
	tok: "Tokyo, Japan",
	mba: "Mumbai, India",
	tor: "Toronto, Canada",
	dca: "Washington DC, USA",
	dfw: "Dallas, Texas, USA",
	ewr: "Newark, New Jersey, USA",
	lon: "London, UK",
	sgp: "Singapore",
	lax: "Los Angeles, California, USA",
	osa: "Osaka, Japan",
	gru: "Sao Paulo",
	bom: "Mumbai, India",
	sin: "Singapore",
	"eu-central-1": "Frankfurt, Germany",
	"us-east-1": "Northern Virginia, USA",
	"us-west-1": "Oregon, USA",
	"ap-southeast-1": "Singapore",
	cloud: "Rivet Cloud",
};

export function getRegionLabel(regionId: string | undefined) {
	return regionId
		? (REGION_LABEL[regionId] ?? REGION_LABEL.unknown)
		: REGION_LABEL.unknown;
}

export function getRegionKey(regionNameId: string | undefined) {
	return regionNameId;
}

export function RegionIcon({
	region = "",
	...props
}: {
	region: string | undefined;
	className?: string;
}) {
	const regionIcon = REGION_ICON[region] ?? REGION_ICON.unknown;

	if (typeof regionIcon === "string") {
		return (
			<AssetImage
				{...props}
				src={`/icons/emoji/${convertEmojiToUriFriendlyString(regionIcon)}.svg`}
			/>
		);
	}

	return <Icon {...props} icon={regionIcon} />;
}
