#!/usr/bin/env python3

# Modifies OpenAPI to support OpenAPI generators with bugs in them

import yaml
import os

FERN_GROUP=os.environ.get('FERN_GROUP')

# Create output dir
print('Creating output dir')
output_dir=f'sdks/{FERN_GROUP}/openapi_compat'
if not os.path.exists(output_dir):
	os.makedirs(output_dir)

# Read spec
print('Reading spec')
with open(f'sdks/{FERN_GROUP}/openapi/openapi.yml', 'r') as f:
	openapi = yaml.safe_load(f.read())

# Modify spec for compatibility
print('Modifying spec for compatibility')
openapi['info']['version'] = '0.0.1'

# Write new spec
print('Writing new spec')
with open(f'{output_dir}/openapi.yml', "w") as f:
	yaml.dump(openapi, f)

print('Done')

