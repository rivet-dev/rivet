#!/usr/bin/env python3

# Modifies OpenAPI to support OpenAPI generators with bugs in them

import yaml
import os

# Create output dir
output_dir='sdks/openapi-compat'
if not os.path.exists(output_dir):
	os.makedirs(output_dir)

# Read spec
with open('sdks/openapi/openapi.yml', 'r') as f:
	openapi = yaml.safe_load(f.read())

# Modify spec for compatibility
openapi['info']['version'] = '0.0.1'

# Write new spec
with open(f'{output_dir}/openapi.yml', "w") as f:
	yaml.dump(openapi, f)
