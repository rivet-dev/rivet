#!/bin/sh
# shellcheck enable=add-default-case
# shellcheck enable=avoid-nullary-conditions
# shellcheck enable=check-unassigned-uppercase
# shellcheck enable=deprecate-which
# shellcheck enable=quote-safe-variables
# shellcheck enable=require-variable-braces
set -eu

rm -rf /tmp/rivet_cli_install
mkdir /tmp/rivet_cli_install
cd /tmp/rivet_cli_install

RIVET_CLI_VERSION="${RIVET_CLI_VERSION:-__VERSION__}"
UNAME="$(uname -s)"
ARCH="$(uname -m)"

# Find asset suffix
if [ "$(printf '%s' "$UNAME" | cut -c 1-6)" = "Darwin" ]; then
	echo
	echo "> Detected macOS"

	if [ "$ARCH" = "x86_64" ]; then
		FILE_NAME="rivet-x86-mac"
	elif [ "$ARCH" = "arm64" ]; then
		FILE_NAME="rivet-aarch64-mac"
	else
		echo "Unknown arch $ARCH" 1>&2
		exit 1
	fi
elif [ "$(printf '%s' "$UNAME" | cut -c 1-5)" = "Linux" ]; then
	echo
	echo "> Detected Linux ($(getconf LONG_BIT) bit)"

	FILE_NAME="rivet-x86-linux"
else
	echo "Unable to determine platform" 1>&2
	exit 1
fi

# Determine install location
set +u
if [ -z "$BIN_DIR" ]; then
	BIN_DIR="/usr/local/bin"
fi
set -u
INSTALL_PATH="$BIN_DIR/rivet"

if [ ! -d "$BIN_DIR" ]; then
    # Find the base parent directory. We're using mkdir -p, which recursively creates directories, so we can't rely on `dirname`.
    CHECK_DIR="$BIN_DIR"
    while [ ! -d "$CHECK_DIR" ] && [ "$CHECK_DIR" != "/" ]; do
        CHECK_DIR=$(dirname "$CHECK_DIR")
    done

    # Check if the directory is writable
    if [ ! -w "$CHECK_DIR" ]; then
        echo
        echo "> Creating directory $BIN_DIR (requires sudo)"
        sudo mkdir -p "$BIN_DIR"
    else
        echo
        echo "> Creating directory $BIN_DIR"
        mkdir -p "$BIN_DIR"
    fi

fi

# Download CLI
URL="https://releases.rivet.gg/rivet/${RIVET_CLI_VERSION}/${FILE_NAME}"
echo
echo "> Downloading $URL"
curl -fsSL "$URL" -o rivet-cli
chmod +x rivet-cli

# Move binary
if [ ! -w "$BIN_DIR" ]; then
    echo
    echo "> Installing rivet to $INSTALL_PATH (requires sudo)"
    sudo mv ./rivet-cli "$INSTALL_PATH"
else
    echo
    echo "> Installing rivet to $INSTALL_PATH"
    mv ./rivet-cli "$INSTALL_PATH"
fi

# Check if path may be incorrect
case ":$PATH:" in
	*:$BIN_DIR:*) ;;
	*) 
		echo "WARNING: $BIN_DIR is not in \$PATH"
		echo "For instructions on how to add it to your PATH, visit:"
		echo "https://opensource.com/article/17/6/set-path-linux"
		;;
esac

echo
echo "> Checking installation"
"$BIN_DIR/rivet" --version

echo
echo "Rivet was installed successfully."
echo "Run 'rivet --help' to get started."
