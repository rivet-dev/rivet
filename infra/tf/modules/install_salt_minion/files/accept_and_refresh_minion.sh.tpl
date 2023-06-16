#!/usr/bin/env bash
set -euf -o pipefail

# https://docs.saltproject.io/salt/install-guide/en/latest/topics/accept-keys.html#accepting-keys
echo '> Checking if key is accepted'
if ! salt-key --list=accepted | grep -E -i '^${name}$'; then
	echo '> Accepting key'
	while ! salt-key --yes --accept '${name}' | grep 'Key for minion ${name} accepted.'; do
		echo '  Key not registered with master'
		sleep 5
	done
else
	echo '  Key already accepted'
fi

# Refresh Salt pillar so we can target the minions using grain
# items when applying Salt states
echo '> Refreshing pillar'
while ! salt '${name}' saltutil.refresh_pillar; do
	echo '  Waiting for Salt minion to connect'
	sleep 5
done

