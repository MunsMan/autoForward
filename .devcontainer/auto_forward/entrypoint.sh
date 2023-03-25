#!/bin/bash

set -e

ls -l /usr/local/share/auto_forward
whoami

/usr/local/share/auto_forward/container &

echo "Container is Listening!"

exec "$@"

