{ stdenv, fetchurl, lib, ... }:

stdenv.mkDerivation rec {
	pname = "clickhouse";
	version = "23.4.2.11";

	src = fetchurl {
		url = "https://github.com/ClickHouse/ClickHouse/releases/download/v${version}-stable/clickhouse-common-static-${version}-amd64.tgz";
		sha256 = "sha256-Y+8/HmXkvaTX3MvkHylb7ZUuaeMcPyL5UXXAh2SI3h8=";
	};

	phases = [ "unpackPhase" "installPhase" ];

	installPhase = ''
		mkdir -p $out/bin
		cp usr/bin/clickhouse $out/bin/
	'';

	meta = with lib; {
		description = "ClickHouse";
		homepage = "https://clickhouse.yandex/";
		license = licenses.asl20;
		platforms = platforms.linux;
	};
}
