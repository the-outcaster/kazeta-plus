#!/bin/bash

# Exit immediately if any command fails.
set -e

# --- Color Definitions for pretty output ---
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

### ===================================================================
###                       PRE-FLIGHT CHECKS
### ===================================================================

echo -e "${GREEN}Starting Kazeta+ Upgrade...${NC}"

# 1. Check for Root Privileges
if [ "$EUID" -ne 0 ]; then
  echo -e "${RED}Error: This script must be run with sudo.${NC}"
  exit 1
fi

# 2. Find Paths
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
DEPLOYMENT_DIR=$(find /frzr_root/deployments -name "kazeta-*" -type d | head -n 1)

if [ -z "$DEPLOYMENT_DIR" ]; then
    echo -e "${RED}Error: Could not find Kazeta installation. Is frzr-unlock running?${NC}"
    exit 1
fi

echo -e "Found Kazeta installation at: ${YELLOW}$DEPLOYMENT_DIR${NC}"
echo "--------------------------------------------------"

### ===================================================================
###                     INTERNET CONNECTIVITY
### ===================================================================

echo -e "${YELLOW}Step 1: Establishing an internet connection...${NC}"

check_connection() {
    ping -c 1 -W 3 8.8.8.8 &> /dev/null
}

if ! check_connection; then
    echo -e "${YELLOW}  -> No internet connection detected. Manual Wi-Fi setup required.${NC}"

    # 1. Verify that the necessary tools exist BEFORE trying to use them.
    if ! command -v wpa_passphrase &> /dev/null || ! command -v wpa_supplicant &> /dev/null || ! command -v dhcpcd &> /dev/null; then
        echo -e "${RED}Error: Critical networking tools (wpa_supplicant, dhcpcd) are missing.${NC}"
        echo -e "${RED}Cannot set up Wi-Fi. Please connect via Ethernet to proceed.${NC}"
        exit 1
    fi

    # Find a wireless interface automatically
    INTERFACE=$(find /sys/class/net -name 'wl*' -printf '%f' -quit)
    if [ -z "$INTERFACE" ]; then
        echo -e "${RED}Error: Could not find a wireless network interface (e.g., wlan0).${NC}"
        exit 1
    fi
    echo "  -> Found wireless interface: ${YELLOW}$INTERFACE${NC}"

    # Get network details from the user
    read -p "  -> Enter your Wi-Fi Network Name (SSID): " SSID
    read -sp "  -> Enter your Wi-Fi Password: " PSK
    echo "" # Newline after password input

    echo "  -> Configuring network..."
    # Generate a temporary config file for wpa_supplicant
    CONF_FILE="/tmp/wpa_supplicant.conf"
    wpa_passphrase "$SSID" "$PSK" > "$CONF_FILE"

    echo "  -> Starting Wi-Fi services..."
    # Ensure the interface is up, but not configured
    ip link set "$INTERFACE" up

    # Kill any old processes to ensure a clean start
    killall wpa_supplicant &>/dev/null || true

    # Start wpa_supplicant in the background
    wpa_supplicant -B -i "$INTERFACE" -c "$CONF_FILE"

    echo "  -> Authenticating (this may take a moment)..."
    sleep 5 # Give it a moment to authenticate

    echo "  -> Obtaining IP address..."
    # Get an IP address using DHCP
    dhcpcd "$INTERFACE"

    sleep 5 # Give it a moment to get an IP

    if ! check_connection; then
        echo -e "${RED}Failed to establish an internet connection. Please check credentials and try again.${NC}"
        exit 1
    fi
    echo -e "${GREEN}  -> Internet connection is now active.${NC}"
else
    echo -e "${GREEN}  -> Internet connection is already active.${NC}"
fi
echo "--------------------------------------------------"

### ===================================================================
###                      SYSTEM PACKAGE UPGRADE
### ===================================================================

echo -e "${YELLOW}Step 2: Installing necessary system packages...${NC}"
pacman -Syy
PACKAGES_TO_INSTALL=("brightnessctl" "keyd" "rsync" "xxhash" "iwd" "networkmanager")
for pkg in "${PACKAGES_TO_INSTALL[@]}"; do
    if ! pacman -Q "$pkg" &>/dev/null; then
        echo "  -> Installing $pkg..."
        pacman -S --noconfirm "$pkg"
    else
        echo "  -> $pkg is already installed."
    fi
done
echo -e "${GREEN}System packages are up to date.${NC}"
echo "--------------------------------------------------"

### ===================================================================
###                         SYSTEM FILE COPY & SERVICES
### ===================================================================

# (The rest of the script remains exactly the same as before)
# It will copy files and enable NetworkManager for a permanent solution.

echo -e "${YELLOW}Step 3: Copying new system files...${NC}"
rsync -av "$SCRIPT_DIR/rootfs/etc/" "$DEPLOYMENT_DIR/etc/"
rsync -av "$SCRIPT_DIR/rootfs/usr/share/" "$DEPLOYMENT_DIR/usr/share/"
echo "  -> Correcting ownership and permissions for sudoers.d..."
SUDOERS_D_DIR="$DEPLOYMENT_DIR/etc/sudoers.d"
if [ -d "$SUDOERS_D_DIR" ]; then
    chown -R root:root "$SUDOERS_D_DIR"
    chmod 755 "$SUDOERS_D_DIR"
    find "$SUDOERS_D_DIR" -type f -exec chmod 440 {} \;
fi
backup_and_copy() {
    local source_file=$1
    local dest_file=$2
    local filename=$(basename "$source_file")
    echo "  -> Processing executable: $filename"
    if [ -f "$dest_file" ]; then mv "$dest_file" "$dest_file.bak"; fi
    cp "$source_file" "$dest_file"
    chmod +x "$dest_file"
}
DEST_BIN_DIR="$DEPLOYMENT_DIR/usr/bin"
for executable in "$SCRIPT_DIR/rootfs/usr/bin/"*; do
    backup_and_copy "$executable" "$DEST_BIN_DIR/$(basename "$executable")"
done
echo -e "${GREEN}System files updated.${NC}"
echo "--------------------------------------------------"

### ===================================================================
###                         ENABLE SERVICES
### ===================================================================

echo -e "${YELLOW}Step 4: Enabling new system services...${NC}"
# Enable NetworkManager for a robust, permanent Wi-Fi solution
SERVICES_TO_ENABLE=("keyd.service" "kazeta-profile-loader.service" "NetworkManager.service")
for service in "${SERVICES_TO_ENABLE[@]}"; do
    echo "  -> Enabling and starting $service..."
    systemctl enable --now "$service"
done
echo -e "${GREEN}Services enabled.${NC}"
echo "--------------------------------------------------"

### ===================================================================
###                       COPY CUSTOM ASSETS
### ===================================================================

echo -e "${YELLOW}Step 5: Copying custom user assets...${NC}"
DEST_ASSET_DIR="$DEPLOYMENT_DIR/home/gamer/.local/share/kazeta-plus"
SOURCE_ASSET_DIR="$SCRIPT_DIR/custom_assets_template"
mkdir -p "$DEST_ASSET_DIR"
if [ -z "$(ls -A $SOURCE_ASSET_DIR 2>/dev/null)" ]; then
    echo "  -> No custom assets found. Skipping."
else
    cp -ruv "$SOURCE_ASSET_DIR/"* "$DEST_ASSET_DIR/"
fi
chown -R 1000:1000 "$DEST_ASSET_DIR"
echo -e "${GREEN}Custom assets processed.${NC}"
echo "--------------------------------------------------"

### ===================================================================
###                             COMPLETE
### ===================================================================

echo -e "${GREEN}Upgrade to Kazeta+ is complete!${NC}"
echo -e "${YELLOW}Please reboot your system now for all changes to take effect.${NC}"
