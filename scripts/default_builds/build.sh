#!/bin/sh
set -euf

tag=$(date -u +%s)

if [ "$#" -eq 0 ]; then
	builds=$(ls infra/default-builds/dockerfiles; ls infra/default-builds/js)
else
	builds="$@"
fi

for build in $builds; do
	build_path="infra/default-builds/dockerfiles/${build}"
	js_build_path="infra/default-builds/js/${build}"

	image="${build}:$tag"

	if [ -d "$build_path" ]; then
		echo
		echo "> $build"
		echo "  * Building"
		docker build --platform linux/amd64 -t "$image" "$build_path"

		echo "  * Saving"
		docker image save --output "infra/default-builds/outputs/${build}.tar" "$image"

		printf "$image" > "infra/default-builds/outputs/${build}-tag.txt"
	elif [ -d "$js_build_path" ]; then
		echo
		echo "> $build"
		echo "  * Building"
		cp "$js_build_path/index.js" "infra/default-builds/outputs/${build}.js"

		echo "  * Writing tag"
		printf "$image" > "infra/default-builds/outputs/${build}-tag.txt"
	else
		echo
		echo "> $build"
		echo "  * Build path does not exist"
	fi
done

ns=$(bolt output namespace)

echo
echo "Deleting old job"
kubectl \
	--kubeconfig "../gen/k8s/kubeconfig/${ns}.yml" \
	-n rivet-service \
	delete \
	--ignore-not-found \
	jobs.batch rivet-build-default-create

echo
echo "Applying to cluster"
bolt up build-default-create

