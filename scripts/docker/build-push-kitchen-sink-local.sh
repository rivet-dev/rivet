#!/usr/bin/env bash
set -euo pipefail

AR_HOSTNAME=${AR_HOSTNAME:-us-east4-docker.pkg.dev}
AR_PROJECT_ID=${AR_PROJECT_ID:-dev-projects-491221}
AR_REPOSITORY=${AR_REPOSITORY:-cloud-run-source-deploy}
IMAGE_NAMESPACE=${IMAGE_NAMESPACE:-rivet-dev-rivet}
IMAGE_NAME=${IMAGE_NAME:-rivet-kitchen-sink}
IMAGE_REPO=${IMAGE_REPO:-"${AR_HOSTNAME}/${AR_PROJECT_ID}/${AR_REPOSITORY}/${IMAGE_NAMESPACE}/${IMAGE_NAME}"}

COMMIT_SHA=${COMMIT_SHA:-$(git rev-parse HEAD)}
DOCKERFILE=${DOCKERFILE:-examples/kitchen-sink/Dockerfile.local}
CONTEXT_DIR=${CONTEXT_DIR:-$(mktemp -d -t rivet-kitchen-sink-image.XXXXXX)}
KEEP_CONTEXT=${KEEP_CONTEXT:-0}
PUSH=${PUSH:-1}

ROOT_DIR=$(git rev-parse --show-toplevel)
APP_DIR="${CONTEXT_DIR}/app"
TARBALL_DIR="${APP_DIR}/tarballs"

PACKAGES=(
	"rivetkit"
	"@rivetkit/react"
	"@rivetkit/framework-base"
	"@rivetkit/sql-loader"
	"@rivetkit/rivetkit-napi"
	"@rivetkit/rivetkit-wasm"
	"@rivetkit/traces"
	"@rivetkit/workflow-engine"
	"@rivetkit/engine-cli"
	"@rivetkit/engine-envoy-protocol"
	"@rivetkit/virtual-websocket"
)

cleanup() {
	if [[ "${KEEP_CONTEXT}" != "1" ]]; then
		rm -rf "${CONTEXT_DIR}"
	fi
}
trap cleanup EXIT

echo "Building kitchen-sink and RivetKit packages on host"
pnpm build --filter=kitchen-sink

echo "Preparing portable Docker context at ${CONTEXT_DIR}"
rm -rf "${CONTEXT_DIR}"
mkdir -p "${APP_DIR}" "${TARBALL_DIR}"

rsync -a --delete \
	--exclude node_modules \
	--exclude .turbo \
	"${ROOT_DIR}/examples/kitchen-sink/" \
	"${APP_DIR}/"

for package in "${PACKAGES[@]}"; do
	pnpm --filter "${package}" pack --pack-destination "${TARBALL_DIR}" >/dev/null
done

APP_DIR="${APP_DIR}" TARBALL_DIR="${TARBALL_DIR}" node <<'NODE'
const { execFileSync } = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");

const appDir = process.env.APP_DIR;
const tarballDir = process.env.TARBALL_DIR;
const packagePath = path.join(appDir, "package.json");
const pkg = JSON.parse(fs.readFileSync(packagePath, "utf8"));
const tarballs = fs.readdirSync(tarballDir).filter((file) => file.endsWith(".tgz"));
const packageSpecs = new Map();

for (const tarball of tarballs) {
	const tarballPath = path.join(tarballDir, tarball);
	const packageJson = JSON.parse(
		execFileSync("tar", ["-xOf", tarballPath, "package/package.json"], {
			encoding: "utf8",
		}),
	);
	packageSpecs.set(packageJson.name, `file:./tarballs/${tarball}`);
}

for (const section of ["dependencies", "devDependencies", "optionalDependencies"]) {
	if (!pkg[section]) continue;
	for (const name of Object.keys(pkg[section])) {
		const spec = packageSpecs.get(name);
		if (spec) pkg[section][name] = spec;
	}
}

pkg.pnpm = pkg.pnpm || {};
pkg.pnpm.overrides = {
	...(pkg.pnpm.overrides || {}),
};
for (const [name, spec] of packageSpecs) {
	pkg.pnpm.overrides[name] = spec;
}

fs.writeFileSync(packagePath, `${JSON.stringify(pkg, null, "\t")}\n`);
NODE

(
	cd "${APP_DIR}"
	pnpm install --no-frozen-lockfile
)

NAPI_PACKAGE_DIR=$(node -e "console.log(require('node:fs').realpathSync(process.argv[1]))" "${APP_DIR}/node_modules/@rivetkit/rivetkit-napi")
shopt -s nullglob
napi_binaries=("${ROOT_DIR}"/rivetkit-typescript/packages/rivetkit-napi/*.node)
shopt -u nullglob
if (( ${#napi_binaries[@]} == 0 )); then
	echo "Missing built rivetkit-napi .node binary. Run the build on a supported host." >&2
	exit 1
fi
cp "${napi_binaries[@]}" "${NAPI_PACKAGE_DIR}/"

rm -rf "${TARBALL_DIR}"

echo "Building ${IMAGE_REPO}:${COMMIT_SHA} and ${IMAGE_REPO}:latest"
docker build \
	-f "${ROOT_DIR}/${DOCKERFILE}" \
	-t "${IMAGE_REPO}:${COMMIT_SHA}" \
	-t "${IMAGE_REPO}:latest" \
	"${CONTEXT_DIR}"

if [[ "${PUSH}" == "1" ]]; then
	echo "Pushing ${IMAGE_REPO}:${COMMIT_SHA}"
	docker push "${IMAGE_REPO}:${COMMIT_SHA}"

	echo "Pushing ${IMAGE_REPO}:latest"
	docker push "${IMAGE_REPO}:latest"
else
	echo "Skipping push because PUSH=${PUSH}"
fi

echo "Done"
