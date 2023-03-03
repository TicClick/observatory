#!/bin/bash
#
# For dedicated user setup, run this under a user who will be controlling the service.
# Prerequisite: https://wiki.archlinux.org/title/systemd/User#Automatic_start-up_of_systemd_user_instances
#
# If you don't have such a user, run:
#   binary_name="observatory"; adduser --disabled-password --gecos "" $binary_name
#
# For system-wide service install, use:
#   systemd_service_dir="/etc/systemd/system"

binary_name="observatory"
systemd_service_dir="${HOME}/.config/systemd/user"
user_flag=""
if [[ "$systemd_service_dir" = "${HOME}"* ]]; then
    systemctl --user --no-pager status || (
        echo "systemd service not running in user mode; run \"sudo loginctl enable-linger $USER\" and reboot" && \
            exit 1
    )

    user_flag="--user"
    echo "export XDG_RUNTIME_DIR=/run/user/$UID" >> ~/.profile
    echo "export DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/$UID/bus" >> ~/.profile
    mkdir -p "$systemd_service_dir"
fi

cat > "$systemd_service_dir/$binary_name.path" <<EOF
[Unit]
Description=Monitor the $binary_name binary for changes

[Path]
PathChanged=/home/$USER/$binary_name
Unit=$binary_name.service

[Install]
WantedBy=default.target
EOF

cat > "$systemd_service_dir/$binary_name.service" <<EOF
[Unit]
Description=osu! wiki helper
Wants=network.target

[Service]
Type=simple
ExecStart=/home/$USER/$binary_name
WorkingDirectory=/home/$USER
Restart=always
RestartSec=15s

[Install]
WantedBy=default.target
EOF

systemctl "$user_flag" daemon-reload && \
    systemctl "$user_flag" enable $binary_name.path $binary_name.service && \
    systemctl "$user_flag" start $binary_name.path $binary_name.service
