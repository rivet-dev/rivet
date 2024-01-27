locals {
	all_monitors_list = flatten([
		for i, group in var.better_uptime_groups:
		[
			for monitor in group.monitors:
			{
				key = "${group.id}-${monitor.id}"
				group_idx = i
				group = group
				monitor = monitor
			}
		]
	])
	all_monitors = {
		for x in local.all_monitors_list:
		x.key => x
	}
}

provider "betteruptime" {
	api_token = module.betteruptime_secrets.values["better_uptime/token"]
}

module "betteruptime_secrets" {
	source = "../modules/secrets"
	keys = [
		"better_uptime/token",
		"rivet/api_status/token",
	]
}

resource "betteruptime_status_page" "status_page" {
	company_name = var.better_uptime.company_name
	company_url = var.better_uptime.company_url
	timezone = "UTC"
	subdomain = var.better_uptime.company_subdomain
	design = "v2"
	layout = "vertical"
	theme = "dark"
}

resource "betteruptime_status_page_section" "status_page_section" {
	count = length(var.better_uptime_groups)

	status_page_id = betteruptime_status_page.status_page.id
	name = var.better_uptime_groups[count.index].name
	position = count.index
}

resource "betteruptime_monitor" "monitor" {
	for_each = local.all_monitors

	url = each.value.monitor.url
	monitor_type = "status"
	request_headers = [
		{
			name = "Authorization"
			value = "Bearer ${module.betteruptime_secrets.values["rivet/api_status/token"]}"
		}
	]
}

resource "betteruptime_status_page_resource" "status_page_resource" {
	for_each = local.all_monitors

	public_name = each.value.monitor.public_name
	resource_id = betteruptime_monitor.monitor[each.key].id
	resource_type = "Monitor"
	status_page_id = betteruptime_status_page.status_page.id
	status_page_section_id = betteruptime_status_page_section.status_page_section[each.value.group_idx].id
	widget_type = "plain"
}
