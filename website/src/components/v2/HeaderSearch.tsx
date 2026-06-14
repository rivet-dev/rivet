import { TypesenseSearch } from "./TypesenseSearch";

export function HeaderSearch({ light = false }: { light?: boolean }) {
	return <TypesenseSearch light={light} />;
}
