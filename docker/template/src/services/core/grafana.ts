import { TemplateContext } from "../../context";
import * as fs from "fs";
import * as path from "path";
import * as yaml from "js-yaml";

export function generateCoreGrafana(context: TemplateContext) {
	const clickhouseHost = context.config.networkMode === "host" ? "127.0.0.1" : "clickhouse";
	// Grafana configuration
	const grafanaIni = `[server]
http_port = 3000
root_url = http://localhost:3100

[security]
admin_user = admin
admin_password = admin

[auth.anonymous]
enabled = true
org_role = Viewer

[dashboards]
default_home_dashboard_path = /var/lib/grafana/dashboards/api.json
`;

	// Datasource configuration for ClickHouse
	const datasourcesConfig = {
		apiVersion: 1,
		datasources: [
			{
				name: "ClickHouse",
				uid: "clickhouse",
				type: "grafana-clickhouse-datasource",
				access: "proxy",
				secureJsonData: {
					password: "default"
				},
				jsonData: {
					version: "2.0.0",
					host: clickhouseHost,
					port: 9300,
					defaultDatabase: "default",
					protocol: "http",
					secure: false,
					username: "default",
					validateSql: true,
					logs: {
						otelEnabled: true,
						otelVersion: "1.2.9",
						defaultDatabase: "otel",
						defaultTable: "otel_logs",
						timeColumn: "TimestampTime",
						messageColumn: "Body",
						levelColumn: "SeverityText"
					},
					traces: {
						otelEnabled: true,
						otelVersion: "1.2.9",
						defaultDatabase: "otel",
						defaultTable: "otel_traces"
					}
				}
			}
		]
	};

	// Dashboard provisioning configuration
	const dashboardsConfig = {
		apiVersion: 1,
		providers: [
			{
				name: "rivet-dashboards",
				orgId: 1,
				folder: "",
				type: "file",
				disableDeletion: false,
				updateIntervalSeconds: 10,
				options: {
					path: "/var/lib/grafana/dashboards"
				}
			}
		]
	};

	// Write configuration files
	context.writeCoreServiceFile("grafana", "grafana.ini", grafanaIni);
	context.writeCoreServiceFile("grafana", "provisioning/datasources/datasources.yaml", yaml.dump(datasourcesConfig));
	context.writeCoreServiceFile("grafana", "provisioning/dashboards/dashboards.yaml", yaml.dump(dashboardsConfig));

	// Copy all dashboard files from the template directory
	const dashboardsDir = path.join(__dirname, "../../../grafana-dashboards");
	const dashboardFiles = fs.readdirSync(dashboardsDir);

	dashboardFiles.forEach(file => {
		if (file.endsWith('.json')) {
			const dashboardContent = fs.readFileSync(path.join(dashboardsDir, file), "utf-8");
			context.writeCoreServiceFile("grafana", `dashboards/${file}`, dashboardContent);
		}
	});
}
