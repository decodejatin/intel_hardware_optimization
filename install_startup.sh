#!/bin/bash
# Run this script with sudo to install the optimization script as a startup service
set -e

if [ "$EUID" -ne 0 ]; then
  echo "Please run this script with sudo"
  exit 1
fi

PROJECT_DIR="/home/jalandhra/Desktop/intel_mac_parity_project"
SCRIPT_PATH="$PROJECT_DIR/optimize.sh"

if [ ! -f "$SCRIPT_PATH" ]; then
    echo "Error: optimize.sh not found in $PROJECT_DIR"
    exit 1
fi

SERVICE_FILE="/etc/systemd/system/intel-mac-parity-tune.service"

echo "Creating systemd service at $SERVICE_FILE..."

cat <<EOF > "$SERVICE_FILE"
[Unit]
Description=Intel Core Ultra 5 125H Performance Tuning
After=multi-user.target

[Service]
Type=oneshot
ExecStart=/bin/bash $SCRIPT_PATH
RemainAfterExit=yes

[Install]
WantedBy=multi-user.target
EOF

echo "Reloading systemd daemon..."
systemctl daemon-reload

echo "Enabling service to run on startup..."
systemctl enable intel-mac-parity-tune.service

echo "Starting service now..."
systemctl start intel-mac-parity-tune.service

echo ""
echo "✅ Startup service installed successfully!"
echo "Your optimizations will now automatically apply every time you boot."
