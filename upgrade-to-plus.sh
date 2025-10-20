#!/bin/bash

# Exit immediately if any command fails.
set -e
# Add pipefail to ensure pipeline failures are caught
set -o pipefail

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
###                  INSTALL LOCAL WI-FI PACKAGES
### ===================================================================
# This step allows users without Ethernet to get Wi-Fi drivers and tools
# installed from a separate, manually placed folder.

echo -e "${YELLOW}Step 1: Checking for and installing local network packages...${NC}"
WIFI_PACK_DIR="$SCRIPT_DIR/kazeta-wifi-pack"

if [ -d "$WIFI_PACK_DIR" ] && [ -n "$(ls -A $WIFI_PACK_DIR/*.pkg.tar.zst 2>/dev/null)" ]; then
    echo "  -> Found local Wi-Fi package folder."
    echo "  -> Installing packages..."
    pacman -U --noconfirm --needed $WIFI_PACK_DIR/*.pkg.tar.zst
    echo -e "${GREEN}  -> Local network packages installed successfully.${NC}"

    # Start the necessary services for nmcli to work.
    echo "  -> Starting network services..."
    systemctl start NetworkManager.service
    # Some configurations of NetworkManager use iwd as a backend, so we start it too.
    systemctl start iwd.service
else
    echo "  -> No local Wi-Fi package folder found. Assuming network tools are present or you have an Ethernet connection."
fi
echo "--------------------------------------------------"


### ===================================================================
###                     INTERNET CONNECTIVITY
### ===================================================================

echo -e "${YELLOW}Step 2: Establishing an internet connection...${NC}"

check_connection() {
    ping -c 1 -W 3 8.8.8.8 &> /dev/null
}

if ! check_connection; then
    echo -e "${YELLOW}  -> No internet connection detected. Starting manual Wi-Fi setup...${NC}"

    if ! command -v nmcli &> /dev/null; then
        echo -e "${RED}Error: 'nmcli' command not found. Cannot set up Wi-Fi.${NC}"
        echo -e "${RED}Please connect via Ethernet or ensure local packages were installed.${NC}"
        exit 1
    fi

    # Give NetworkManager a moment to start and detect devices.
    echo "  -> Waiting for Wi-Fi hardware to initialize..."
    sleep 3

    echo "  -> Scanning for networks..."
    nmcli device wifi rescan

    nmcli --terse --fields SSID,SIGNAL device wifi list | head -n 10

    read -p "  -> Enter your Wi-Fi Network Name (SSID): " SSID
    read -sp "  -> Enter your Wi-Fi Password: " PSK
    echo ""

    echo "  -> Connecting..."
    nmcli device wifi connect "$SSID" password "$PSK"

    sleep 5

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
###         SYSTEM PACKAGE UPGRADE & BUILD TOOLS
### ===================================================================

echo -e "${YELLOW}Step 3: Installing/updating system packages and build tools...${NC}"
pacman -Syy

# -- ADDED -- base-devel, dkms, linux-headers are needed for the DKMS module
# NOTE: Ensure 'linux-headers' matches the kernel Kazeta+ uses. If it's a custom
# kernel, you might need a different headers package.
PACKAGES_TO_INSTALL=(
    "brightnessctl" "keyd" "rsync" "xxhash" "iwd" "networkmanager"
    "ffmpeg" "unzip" "bluez" "bluez-utils"
    "base-devel" "dkms" "linux-headers"
)

# Install required packages (including build dependencies)
for pkg in "${PACKAGES_TO_INSTALL[@]}"; do
    # Using --needed ensures we don't reinstall if already present & up-to-date
    echo "  -> Ensuring $pkg is installed..."
    pacman -S --noconfirm --needed "$pkg"
done
echo -e "${GREEN}System packages are up to date.${NC}"
echo "--------------------------------------------------"


### ===================================================================
###         BUILD AND INSTALL GC ADAPTER OVERCLOCK MODULE (DKMS)
### ===================================================================

echo -e "${YELLOW}Step 4: Building and installing GCC overclocking module...${NC}"
GC_MODULE_SRC_DIR="$SCRIPT_DIR/aur-pkgs/gcadapter-oc-dkms"

if [ -d "$GC_MODULE_SRC_DIR" ] && [ -f "$GC_MODULE_SRC_DIR/PKGBUILD" ]; then
    echo "  -> Found source directory for gcadapter-oc-dkms."
    # Use pushd/popd to safely change directory and return
    pushd "$GC_MODULE_SRC_DIR" > /dev/null

    # Build and install the package using makepkg.
    # -s: Syncs dependencies (runtime deps, should be minimal for a kmod)
    # -i: Installs the package after building
    # Since the whole script runs as root, makepkg will install system-wide.
    echo "  -> Running makepkg..."
    makepkg -si --noconfirm

    popd > /dev/null
    echo -e "${GREEN}  -> GCC overclocking module installed successfully.${NC}"
else
    echo -e "${YELLOW}  -> Source directory for gcadapter-oc-dkms not found. Skipping module installation.${NC}"
fi
echo "--------------------------------------------------"


### ===================================================================
###                 SYSTEM FILE COPY & SERVICES
### ===================================================================

echo -e "${YELLOW}Step 5: Copying new system files...${NC}"
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
###                       ENABLE SERVICES
### ===================================================================

echo -e "${YELLOW}Step 6: Enabling new system services...${NC}"
# -- CHANGED -- Added "bluetooth.service"
SERVICES_TO_ENABLE=("keyd.service" "kazeta-profile-loader.service" "NetworkManager.service" "iwd.service" "bluetooth.service")
for service in "${SERVICES_TO_ENABLE[@]}"; do
    echo "  -> Enabling and starting $service..."
    systemctl enable --now "$service"
done
echo -e "${GREEN}Services enabled.${NC}"
echo "--------------------------------------------------"

### ===================================================================
###                    COPY CUSTOM ASSETS
### ===================================================================

echo -e "${YELLOW}Step 7: Copying custom user assets...${NC}"
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
###                           COMPLETE
### ===================================================================

echo -e "${GREEN}Upgrade to Kazeta+ is complete!${NC}"
echo -e "${YELLOW}Please reboot your system now for all changes to take effect.${NC}"

