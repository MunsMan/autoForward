#!/usr/bin/env bash
USERNAME="${USERNAME:-"${_REMOTE_USER:-"automatic"}"}"
PORT="${PORT:-"28258"}"


#Functions

detect_user() {
    local user_variable_name=${1:-username}
    local possible_users=("vscode" "node" "codespace" "$(awk -v val=1000 -F ":" '$3==val{print $1}' /etc/passwd)")
    if [ "${!user_variable_name}" = "auto" ] || [ "${!user_variable_name}" = "automatic" ]; then
        declare -g ${user_variable_name}=""
        for current_user in ${possible_users[@]}; do
            if id -u "${current_user}" > /dev/null 2>&1; then
                declare -g ${user_variable_name}="${current_user}"
                break
            fi
        done
    fi
    if [ "${!user_variable_name}" = "" ] || [ "${!user_variable_name}" = "none" ] || ! id -u "${!user_variable_name}" > /dev/null 2>&1; then
        declare -g ${user_variable_name}=root
    fi
}



# Script

arch=$(uname -m)
echo "Determine the Docker Container Architektur: $arch"

detect_user USERNAME

HOME_DIR="$(eval echo ~${USERNAME})"

DOWNLOAD_DIR="/tmp/auto_forward"
PROJECT_DIR="/usr/local/share/auto_forward"
BINARY="container"

mkdir "${DOWNLOAD_DIR}"

# Downloading Binary File
curl -sL https://github.com/munsman/autoForward/releases/latest/download/container_x86_64 >> "$DOWNLOAD_DIR/$BINARY"



# Setting up the local Feature Directory
mkdir "${PROJECT_DIR}"

tee -a "${PROJECT_DIR}/entrypoint.sh" > /dev/null \
<< EOF
#!/bin/bash

set -e

PORT=${PORT}

EOF


tee -a "${PROJECT_DIR}/entrypoint.sh" > /dev/null \
<< 'EOF'

ls -l /usr/local/share/auto_forward
whoami

echo "${PORT}"

/usr/local/share/auto_forward/container "${PORT}"&

echo "Container is Listening!"

exec "$@"
EOF

cd "${PROJECT_DIR}"
mv "${DOWNLOAD_DIR}/${BINARY}" "${PROJECT_DIR}/${BINARY}"
echo | ls
chmod +x "${BINARY}"
chmod +x entrypoint.sh

echo "autoForward setup script has completed!"