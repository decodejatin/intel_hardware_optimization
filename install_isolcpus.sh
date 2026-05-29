#!/bin/bash
# Safely injects isolcpus=0-7 into GRUB_CMDLINE_LINUX_DEFAULT
set -e

if [ "$EUID" -ne 0 ]; then
  echo "Please run this script with sudo"
  exit 1
fi

GRUB_FILE="/etc/default/grub"

if grep -q "isolcpus" "$GRUB_FILE"; then
  echo "isolcpus is already in $GRUB_FILE. Please remove it manually before running this script."
  exit 1
fi

echo "Backing up $GRUB_FILE to $GRUB_FILE.bak"
cp "$GRUB_FILE" "$GRUB_FILE.bak"

echo "Updating GRUB configuration..."
# This carefully appends isolcpus=0-7 inside the quotes of GRUB_CMDLINE_LINUX_DEFAULT
sed -i 's/GRUB_CMDLINE_LINUX_DEFAULT="\(.*\)"/GRUB_CMDLINE_LINUX_DEFAULT="\1 isolcpus=0-7"/' "$GRUB_FILE"

echo "Applying changes with update-grub..."
update-grub

echo ""
echo "✅ isolcpus configured successfully!"
echo "CPUs 0-7 (P-Cores) will be completely hidden from the Linux OS."
echo "⚠️  You MUST REBOOT your system for this to take effect."
