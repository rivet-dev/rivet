# Create trafficserver user
if ! id -u "trafficserver" &>/dev/null; then
	useradd -r -s /bin/false trafficserver
fi

# Create required dirs
mkdir -p /etc/trafficserver /var/cache/trafficserver /run/trafficserver /var/log/trafficserver

# Write config
__CONFIG__

# Change owner
chown -R trafficserver:trafficserver /etc/trafficserver /var/cache/trafficserver /run/trafficserver /var/log/trafficserver

cat << EOF > /etc/systemd/system/trafficserver.service
[Unit]
Description=Apache Traffic Server
After=docker.service
Requires=docker.service

[Service]
TimeoutStartSec=0
ExecStartPre=-/usr/bin/docker kill trafficserver
ExecStartPre=-/usr/bin/docker rm trafficserver
ExecStartPre=/usr/bin/docker pull "__IMAGE__"
ExecStart=/usr/bin/docker run --rm --name trafficserver \
	--user "$(id -u trafficserver):$(id -g trafficserver)" \
	--volume=/etc/trafficserver:/etc/trafficserver \
	--volume=/var/cache/trafficserver:/var/cache/trafficserver \
	--volume=/run/trafficserver:/run/trafficserver \
	--volume=/var/log/trafficserver:/var/log/trafficserver \
	--network host \
	"__IMAGE__"
ExecStop=/usr/bin/docker stop trafficserver

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable trafficserver
systemctl start trafficserver

