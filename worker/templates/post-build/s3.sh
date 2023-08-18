#!/bin/sh

set -eu
set -f
export IFS=' '

echo "Uploading paths" $OUT_PATHS
export AWS_ACCESS_KEY_ID="{{ key }}"
export AWS_SECRET_ACCESS_KEY="{{ secret }}"
exec /nix/var/nix/profiles/default/bin/nix copy -v --to '{{ cache_uri }}' $OUT_PATHS
