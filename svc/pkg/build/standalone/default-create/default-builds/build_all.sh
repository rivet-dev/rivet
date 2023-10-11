#!/bin/sh
set -euf

tag=$(date -u +%s)

for build in $(ls dockerfiles); do
	build_path="dockerfiles/${build}"

	build_name=${build}
	image="${build_name}:$tag"

	echo
	echo "> $build_name"
	echo "  * Building"
	docker build -t "$image" "$build_path"

	echo "  * Saving"
	docker image save --output "outputs/${build}.tar" "$image"

	printf "$image" > "outputs/${build_name}-tag.txt"
done

echo
echo "New build tag: $tag"

