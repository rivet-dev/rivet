#!/bin/sh
set -euf

tag=$(date -u +%s)

if [ "$#" -eq 0 ]; then
	builds=$(ls infra/default-builds/dockerfiles)
else
	builds="$@"
fi

for build in $builds; do
	build_path="infra/default-builds/dockerfiles/${build}"

	image="${build}:$tag"

	echo
	echo "> $build"
	echo "  * Building"
	docker build -t "$image" "$build_path"

	echo "  * Saving"
	docker image save --output "infra/default-builds/outputs/${build}.tar" "$image"

	printf "$image" > "infra/default-builds/outputs/${build}-tag.txt"
done

echo
echo "Deleting old job"

echo
echo "Applying to cluster"
bolt up build-default-create

