#!/bin/sh

# Kaniko args are passed as a single string and 
# are split by the entrypoint script.

# Keep in sync with ci-manager/common.ts
UNIT_SEP_CHAR=$'\x1F'

# We split the string by the unit separator character
# into an array of words ($@), which preserves any 
# spaces in the arguments.
export IFS=$UNIT_SEP_CHAR
set -- $KANIKO_ARGS

# For build args, values containing spaces are not natively supported by
# Kaniko, they recommend setting IFS to null before command. 
export IFS=''
/kaniko/executor $@