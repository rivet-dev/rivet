# echo "__GHCR_PASSWORD__" | docker login ghcr.io --username "__GHCR_USERNAME__" --password-stdin 

# Create trafficserver user
if ! id -u "trafficserver" &>/dev/null; then
	useradd -r -s /bin/false trafficserver
fi

# Create volumes
for x in /etc/trafficserver /var/cache/trafficserver /var/log/trafficserver; do
	mkdir -p $x
	chown -R trafficserver:trafficserver $x
	chmod -R 770 $x
done

__CONFIG__

chmod -R 770 /etc/trafficserver

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

