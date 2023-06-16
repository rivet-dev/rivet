#!/usr/bin/env bash
set -euf

check_nebula_version () {
	nebula --version | grep -E -i '^Version: ${version}$'
}

# Update config file permission
chmod -R 400 /etc/nebula/config.yaml

echo '> Checking Nebula version'
if ! check_nebula_version; then
	nebula_version_updated=true
	
	echo '> Installing Nebula ${version}'
	extract_path="/opt/nebula_${version}/"

	# Remove any previously attempted extraction
	rm -rf "$extract_path"

	mkdir "$extract_path"
	(
		cd "$extract_path"
		download_url='https://github.com/slackhq/nebula/releases/download/v${version}/nebula-linux-amd64.tar.gz'
		echo "> Downloading binary ($download_url)"
		curl -L -O "$download_url"
		ls
		du -h nebula-linux-amd64.tar.gz
		tar xf nebula-linux-amd64.tar.gz

		echo '> Installing binary'
		rm -f /usr/bin/nebula
		cp ./nebula /usr/bin/nebula
		cp ./nebula-cert /usr/bin/nebula-cert
		
		echo '> Validating version'
		if check_nebula_version; then
			echo '  Version is valid'
		else
			echo '  Version is not valid'
			exit 1
		fi
	)
else
	nebula_version_updated=false
	echo '  Nebula ${version} already installed'
fi


if systemctl --all --type service | grep -Fq 'nebula'; then    
	if [ "$nebula_version_updated" = true ]; then
		echo '> Restarting service'
		systemctl restart nebula || (echo "Failed to restart Nebula." && exit 1)
	else
		echo '> Reloading service'
		systemctl reload nebula || (echo "Failed to reload Nebula." && exit 1)
	fi
else
	echo '> Writing service'
	cat > /etc/systemd/system/nebula.service <<'EOF'
[Unit]
Description=Nebula
Wants=basic.target network-online.target nss-lookup.target time-sync.target
After=basic.target network.target network-online.target
Before=sshd.service

[Service]
Type=simple
User=root
Group=root
ExecReload=/bin/kill -HUP $MAINPID
ExecStart=nebula -config /etc/nebula/config.yaml
SyslogIdentifier=nebula
Restart=always
RestartSec=2

[Install]
WantedBy=multi-user.target
EOF
	systemctl daemon-reload

	echo '> Starting service'
	systemctl start nebula
	systemctl enable nebula
fi

# Wait and then check that Nebula is running successfully
sleep 2
echo '> Checking Nebula status'
systemctl status --no-pager nebula

