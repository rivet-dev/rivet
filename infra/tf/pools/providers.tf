provider "linode" {
	token = module.secrets.values["linode/token"]
}
