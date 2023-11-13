#!/usr/bin/env python3

# Modifies OpenAPI to support OpenAPI generators with bugs in them

import yaml
import os

# Create output dir
output_dir='gen/openapi/internal/spec_compat'
if not os.path.exists(output_dir):
	os.makedirs(output_dir)

# Read spec
with open('gen/openapi/internal/spec/openapi.yml', 'r') as f:
	openapi = yaml.safe_load(f.read())

# Modify spec for compatability
openapi['info']['version'] = '0.0.1'
openapi['components']['schemas']['PortalNotificationUnregisterService'].pop('enum')
openapi['components']['schemas']['CloudGamesLogStream'].pop('enum')

# Write new spec
with open(f'{output_dir}/openapi.yml', "w") as f:
	yaml.dump(openapi, f)
