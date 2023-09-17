echo "__GHCR_PASSWORD__" | docker login ghcr.io --username "__GHCR_USERNAME__" --password-stdin 

# Create volumes
mkdir -p /etc/trafficserver
mkdir -p /etc/trafficserver-s3-auth
mkdir -p /var/cache/trafficserver

__CONFIG__

cat << 'EOF' > /etc/systemd/system/trafficserver.service
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
	--volume=/etc/trafficserver:/etc/trafficserver \
	--volume=/var/cache/trafficserver:/var/cache/trafficserver \
	--publish 8080:8080/tcp \
	"__IMAGE__"
ExecStop=/usr/bin/docker stop trafficserver

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable trafficserver
systemctl start trafficserver

