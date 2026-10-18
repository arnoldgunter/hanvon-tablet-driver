#!/usr/bin/env bash
set -euo pipefail

if [[ "${EUID}" -eq 0 ]]; then
  printf 'Do not run this installer with sudo. Run it as your normal user; sudo is used only for udev rules.\n' >&2
  exit 1
fi

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
systemctl --user cat hanvon-tablet-driver.service >/dev/null
systemctl --user --quiet enable --now hanvon-tablet-driver.service

printf 'Hanvon tablet driver installed and started in the background.\n'
printf 'Follow logs with: journalctl --user -u hanvon-tablet-driver.service -f\n'
