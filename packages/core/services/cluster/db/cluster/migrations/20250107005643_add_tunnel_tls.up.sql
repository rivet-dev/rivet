CREATE TABLE tunnel_tls (
	_id INT PRIMARY KEY, -- Solely to allow ON CONFLICT, there should only be 1 row in this table
	cert_pem TEXT,
	private_key_pem TEXT,
	root_ca_cert_pem TEXT,
	state INT NOT NULL, -- cluster::types::TlsState
	expire_ts INT NOT NULL
);
