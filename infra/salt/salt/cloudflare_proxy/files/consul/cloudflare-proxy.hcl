services {
	name = "cloudflare-proxy"
	tags = ["traefik"]

	port = 9060
	checks = [
		{
			name = "Reachable on 9060"
			tcp = "127.0.0.1:9060"
			interval = "10s"
			timeout  = "1s"
		}
	]
}

