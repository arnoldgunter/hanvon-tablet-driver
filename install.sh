#!/usr/bin/env bash
set -euo pipefail

install_dir="${HOME}/.local/bin"
service_dir="${XDG_CONFIG_HOME:-${HOME}/.config}/systemd/user"
script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"

install -Dm755 "${script_dir}/hanvon-tablet-driver" \
  "${install_dir}/hanvon-tablet-driver"
install -Dm644 "${script_dir}/deploy/hanvon-tablet-driver.service" \
  "${service_dir}/hanvon-tablet-driver.service"

sudo install -Dm644 "${script_dir}/deploy/70-hanvon-tablet.rules" \
  /etc/udev/rules.d/70-hanvon-tablet.rules
sudo udevadm control --reload-rules
sudo udevadm trigger

systemctl --user daemon-reload
systemctl --user enable --now hanvon-tablet-driver.service

printf 'Hanvon driver installed and started.\\n'
printf 'Logs: journalctl --user -u hanvon-tablet-driver.service -f\\n'
