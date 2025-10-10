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
  echo "Please run 'sudo ./upgrade-to-plus.sh'"
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
###                      SYSTEM PACKAGE UPGRADE
### ===================================================================

echo -e "${YELLOW}Step 1: Installing necessary system packages...${NC}"
pacman -Syy
PACKAGES_TO_INSTALL=("brightnessctl" "keyd" "rsync" "xxhash")
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
###                        SYSTEM FILE COPY
### ===================================================================

echo -e "${YELLOW}Step 2: Copying new system files...${NC}"
# Use rsync to copy all non-executable config files and create directories
rsync -av "$SCRIPT_DIR/rootfs/etc/" "$DEPLOYMENT_DIR/etc/"
rsync -av "$SCRIPT_DIR/rootfs/usr/share/" "$DEPLOYMENT_DIR/usr/share/"

# --- ADD THIS BLOCK TO FIX OWNERSHIP AND PERMISSIONS ---
echo "  -> Correcting ownership and permissions for sudoers.d..."
SUDOERS_D_DIR="$DEPLOYMENT_DIR/etc/sudoers.d"
if [ -d "$SUDOERS_D_DIR" ]; then
    chown -R root:root "$SUDOERS_D_DIR"
    chmod 755 "$SUDOERS_D_DIR"
    # The file(s) inside must be read-only by root
    find "$SUDOERS_D_DIR" -type f -exec chmod 440 {} \;
fi
# ----------------------------------------------------

# Explicitly copy executables using a backup function
backup_and_copy() {
    local source_file=$1
    local dest_file=$2
    local filename=$(basename "$source_file")

    echo "  -> Processing executable: $filename"
    if [ -f "$dest_file" ]; then mv "$dest_file" "$dest_file.bak"; fi
    cp "$source_file" "$dest_file"
    chmod +x "$dest_file"
}

# Loop through all executables in our source and copy them
DEST_BIN_DIR="$DEPLOYMENT_DIR/usr/bin"
for executable in "$SCRIPT_DIR/rootfs/usr/bin/"*; do
    backup_and_copy "$executable" "$DEST_BIN_DIR/$(basename "$executable")"
done

echo -e "${GREEN}System files updated.${NC}"
echo "--------------------------------------------------"

### ===================================================================
###                       ENABLE SERVICES
### ===================================================================

echo -e "${YELLOW}Step 3: Enabling new system services...${NC}"
SERVICES_TO_ENABLE=("keyd.service" "kazeta-profile-loader.service")
for service in "${SERVICES_TO_ENABLE[@]}"; do
    echo "  -> Enabling and starting $service..."
    systemctl enable --now "$service"
done
echo -e "${GREEN}Services enabled.${NC}"
echo "--------------------------------------------------"

### ===================================================================
###                      COPY CUSTOM ASSETS
### ===================================================================

echo -e "${YELLOW}Step 4: Copying custom user assets...${NC}"
# ... (This section remains the same)
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
###                          COMPLETE
### ===================================================================

echo -e "${GREEN}Upgrade to Kazeta+ is complete!${NC}"
echo -e "${YELLOW}Please reboot your system now for all changes to take effect.${NC}"
