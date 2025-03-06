ALTER TABLE servers_cloudflare
	ADD api_dns_record_id TEXT;

ALTER TABLE datacenter_tls
	ADD COLUMN api_cert_pem TEXT,
	ADD COLUMN api_private_key_pem TEXT;
