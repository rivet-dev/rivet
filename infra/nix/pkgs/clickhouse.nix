{ stdenv, fetchurl, lib, ... }:

stdenv.mkDerivation rec {
	pname = "clickhouse";
	version = "23.7.2.25";

	src = fetchurl {
		url = "https://github.com/ClickHouse/ClickHouse/releases/download/v${version}-stable/clickhouse-common-static-${version}-amd64.tgz";
		sha256 = "sha256-6EY8Iw+H2ebSK5RvuDlCoM6i4U3L39eJ+WXDruWDXic=";
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
