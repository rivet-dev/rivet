locals {
	# Cloudflare client cert used for mTLS. See
	# https://developers.cloudflare.com/ssl/origin-configuration/authenticated-origin-pull#zone-level--cloudflare-certificate
	cloudflare_ca_cert = <<-EOF
		-----BEGIN CERTIFICATE-----
		MIIGCjCCA/KgAwIBAgIIV5G6lVbCLmEwDQYJKoZIhvcNAQENBQAwgZAxCzAJBgNV
		BAYTAlVTMRkwFwYDVQQKExBDbG91ZEZsYXJlLCBJbmMuMRQwEgYDVQQLEwtPcmln
		aW4gUHVsbDEWMBQGA1UEBxMNU2FuIEZyYW5jaXNjbzETMBEGA1UECBMKQ2FsaWZv
		cm5pYTEjMCEGA1UEAxMab3JpZ2luLXB1bGwuY2xvdWRmbGFyZS5uZXQwHhcNMTkx
		MDEwMTg0NTAwWhcNMjkxMTAxMTcwMDAwWjCBkDELMAkGA1UEBhMCVVMxGTAXBgNV
		BAoTEENsb3VkRmxhcmUsIEluYy4xFDASBgNVBAsTC09yaWdpbiBQdWxsMRYwFAYD
		VQQHEw1TYW4gRnJhbmNpc2NvMRMwEQYDVQQIEwpDYWxpZm9ybmlhMSMwIQYDVQQD
		ExpvcmlnaW4tcHVsbC5jbG91ZGZsYXJlLm5ldDCCAiIwDQYJKoZIhvcNAQEBBQAD
		ggIPADCCAgoCggIBAN2y2zojYfl0bKfhp0AJBFeV+jQqbCw3sHmvEPwLmqDLqynI
		42tZXR5y914ZB9ZrwbL/K5O46exd/LujJnV2b3dzcx5rtiQzso0xzljqbnbQT20e
		ihx/WrF4OkZKydZzsdaJsWAPuplDH5P7J82q3re88jQdgE5hqjqFZ3clCG7lxoBw
		hLaazm3NJJlUfzdk97ouRvnFGAuXd5cQVx8jYOOeU60sWqmMe4QHdOvpqB91bJoY
		QSKVFjUgHeTpN8tNpKJfb9LIn3pun3bC9NKNHtRKMNX3Kl/sAPq7q/AlndvA2Kw3
		Dkum2mHQUGdzVHqcOgea9BGjLK2h7SuX93zTWL02u799dr6Xkrad/WShHchfjjRn
		aL35niJUDr02YJtPgxWObsrfOU63B8juLUphW/4BOjjJyAG5l9j1//aUGEi/sEe5
		lqVv0P78QrxoxR+MMXiJwQab5FB8TG/ac6mRHgF9CmkX90uaRh+OC07XjTdfSKGR
		PpM9hB2ZhLol/nf8qmoLdoD5HvODZuKu2+muKeVHXgw2/A6wM7OwrinxZiyBk5Hh
		CvaADH7PZpU6z/zv5NU5HSvXiKtCzFuDu4/Zfi34RfHXeCUfHAb4KfNRXJwMsxUa
		+4ZpSAX2G6RnGU5meuXpU5/V+DQJp/e69XyyY6RXDoMywaEFlIlXBqjRRA2pAgMB
		AAGjZjBkMA4GA1UdDwEB/wQEAwIBBjASBgNVHRMBAf8ECDAGAQH/AgECMB0GA1Ud
		DgQWBBRDWUsraYuA4REzalfNVzjann3F6zAfBgNVHSMEGDAWgBRDWUsraYuA4REz
		alfNVzjann3F6zANBgkqhkiG9w0BAQ0FAAOCAgEAkQ+T9nqcSlAuW/90DeYmQOW1
		QhqOor5psBEGvxbNGV2hdLJY8h6QUq48BCevcMChg/L1CkznBNI40i3/6heDn3IS
		zVEwXKf34pPFCACWVMZxbQjkNRTiH8iRur9EsaNQ5oXCPJkhwg2+IFyoPAAYURoX
		VcI9SCDUa45clmYHJ/XYwV1icGVI8/9b2JUqklnOTa5tugwIUi5sTfipNcJXHhgz
		6BKYDl0/UP0lLKbsUETXeTGDiDpxZYIgbcFrRDDkHC6BSvdWVEiH5b9mH2BON60z
		0O0j8EEKTwi9jnafVtZQXP/D8yoVowdFDjXcKkOPF/1gIh9qrFR6GdoPVgB3SkLc
		5ulBqZaCHm563jsvWb/kXJnlFxW+1bsO9BDD6DweBcGdNurgmH625wBXksSdD7y/
		fakk8DagjbjKShYlPEFOAqEcliwjF45eabL0t27MJV61O/jHzHL3dknXeE4BDa2j
		bA+JbyJeUMtU7KMsxvx82RmhqBEJJDBCJ3scVptvhDMRrtqDBW5JShxoAOcpFQGm
		iYWicn46nPDjgTU0bX1ZPpTpryXbvciVL5RkVBuyX2ntcOLDPlZWgxZCBp96x07F
		AnOzKgZk4RzZPNAxCXERVxajn/FLcOhglVAKo5H0ac+AitlQ0ip55D2/mf8o72tM
		fVQ6VpyjEXdiIXWUq/o=
		-----END CERTIFICATE-----
		EOF
}

# MARK: Cloudflare origin cert (rivet.gg)
# See https://developers.cloudflare.com/ssl/origin-configuration/authenticated-origin-pull#zone-level--cloudflare-certificate
resource "tls_private_key" "cf_origin_rivet_gg" {
	algorithm = "RSA"
}

resource "tls_cert_request" "cf_origin_rivet_gg" {
	private_key_pem = tls_private_key.cf_origin_rivet_gg.private_key_pem

	subject {
		common_name  = ""
		organization = "Rivet Gaming, LLC"
	}
}

resource "cloudflare_origin_ca_certificate" "rivet_gg" {
	csr = tls_cert_request.cf_origin_rivet_gg.cert_request_pem
	hostnames = flatten([
		[
			"*.${var.domain_main}",
			"${var.domain_main}",
			"*.api.${var.domain_main}",
			"api.${var.domain_main}",
		],
		var.opengb_enabled ? [
			"*.opengb.${var.domain_main}",
			"db.opengb-internal.${var.domain_main}"
		] : []
	])
	request_type = "origin-rsa"
	requested_validity = 15 * 365
}


# Must be created in every namespace it is used in
resource "kubernetes_secret" "ingress_tls_cert" {
	for_each = toset(flatten([
		["traefik", "rivet-service"],
		var.imagor_enabled ? ["imagor"] : [], 
		local.has_minio ? ["minio"] : []
	]))

	metadata {
		name = "ingress-tls-cloudflare-cert"
		namespace = each.value
	}

	type = "kubernetes.io/tls"

	data = {
		"tls.crt" = cloudflare_origin_ca_certificate.rivet_gg.certificate
		"tls.key" = tls_private_key.cf_origin_rivet_gg.private_key_pem
	}
}

resource "kubernetes_secret" "ingress_tls_ca_cert" {
	for_each = toset(flatten([
		["traefik", "rivet-service"],
		var.imagor_enabled ? ["imagor"] : []
	]))

	metadata {
		name = "ingress-tls-cloudflare-ca-cert"
		namespace = each.value
	}

	data = {
		"tls.ca" = local.cloudflare_ca_cert
	}
}

