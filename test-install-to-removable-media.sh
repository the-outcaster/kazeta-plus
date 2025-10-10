#! /bin/bash

# Exit immediately if a command exits with a non-zero status.
set -e

# --- Color Definitions ---
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# --- Check for Root Privileges ---
if [ "$EUID" -ne 0 ]; then
  echo -e "${RED}Error: This script must be run with sudo.${NC}"
  echo "Please try again using 'sudo ./install-bios.sh'"
  exit 1
fi

echo -e "${GREEN}Starting Kazeta+ BIOS Installer...${NC}"

# --- Path Definitions ---
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

# Source paths
SOURCE_BIOS_FILE="$SCRIPT_DIR/bios/target/debug/kazeta-bios"
SOURCE_SCRIPTS_DIR="$SCRIPT_DIR/rootfs/usr/bin"
# NEW: Source path for the sudoers file
SOURCE_SUDOERS_FILE="$SCRIPT_DIR/rootfs/etc/sudoers"

# Dynamically find the Kazeta installation directories
echo -e "${YELLOW}Searching for Kazeta installation directory...${NC}"
DEST_BIN_DIR=$(find /run/media -type d -path "*/frzr_root/deployments/kazeta-*/usr/bin" 2>/dev/null | head -n 1)

# --- Pre-flight Checks ---
if [ -z "$DEST_BIN_DIR" ]; then
    echo -e "${RED}Error: Could not find a Kazeta installation directory.${NC}"
    echo "Please ensure your MicroSD card is mounted."
    exit 1
fi

# NEW: Derive all other destination directories from the found bin directory
DEPLOYMENT_ROOT=$(dirname "$(dirname "$DEST_BIN_DIR")")
DEST_ETC_DIR="$DEPLOYMENT_ROOT/etc"
DEST_ASSET_DIR="$DEPLOYMENT_ROOT/home/gamer/.local/share/kazeta-plus"

if [ ! -f "$SOURCE_BIOS_FILE" ]; then
    echo -e "${RED}Error: BIOS binary not found at '$SOURCE_BIOS_FILE'.${NC}"
    echo "Please compile the BIOS first by running 'cargo build' in the 'bios' directory."
    exit 1
fi

echo -e "Found deployment root: ${YELLOW}$DEPLOYMENT_ROOT${NC}"
echo "--------------------------------------------------"

# NEW: Ensure asset destination directory exists
echo -e "Ensuring asset directory exists..."
mkdir -p "$DEST_ASSET_DIR"
echo -e "  -> ${GREEN}Done.${NC}"

# --- Reusable Function to Backup and Copy a File ---
backup_and_copy() {
    local source_file=$1
    local dest_dir=$2
    local filename=$(basename "$source_file")
    local dest_file="$dest_dir/$filename"

    echo -e "Processing ${YELLOW}$filename${NC}..."

    if [ -f "$dest_file" ]; then
        echo "  -> Backing up existing file to ${filename}.bak"
        mv "$dest_file" "${dest_file}.bak"
    fi

    echo "  -> Copying new file to destination"
    cp "$source_file" "$dest_file"
    chmod +x "$dest_file"
    echo -e "  -> ${GREEN}Done.${NC}"
}

# --- Main Execution ---
# Copy executables
backup_and_copy "$SOURCE_BIOS_FILE" "$DEST_BIN_DIR"
backup_and_copy "$SOURCE_SCRIPTS_DIR/kazeta" "$DEST_BIN_DIR"
backup_and_copy "$SOURCE_SCRIPTS_DIR/kazeta-session" "$DEST_BIN_DIR"

# NEW: Copy the sudoers file with backup and strict permissions
echo -e "Processing ${YELLOW}sudoers file${NC}..."
if [ -f "$DEST_ETC_DIR/sudoers" ]; then
    echo "  -> Backing up existing sudoers file"
    mv "$DEST_ETC_DIR/sudoers" "$DEST_ETC_DIR/sudoers.bak"
fi
echo "  -> Copying new sudoers file"
cp "$SOURCE_SUDOERS_FILE" "$DEST_ETC_DIR/sudoers"
echo "  -> Setting permissions for sudoers"
chmod 0440 "$DEST_ETC_DIR/sudoers"
echo -e "  -> ${GREEN}Done.${NC}"

# NEW: Copy all asset directories
echo -e "Processing ${YELLOW}custom assets${NC}..."
ASSET_TYPES=("backgrounds" "bgm" "fonts" "logos" "sfx")
for asset_type in "${ASSET_TYPES[@]}"; do
    source_dir="$SCRIPT_DIR/bios/$asset_type"
    if [ -d "$source_dir" ]; then
        echo "  -> Copying '$asset_type' directory..."
        # Remove the old directory on the destination to ensure a clean sync
        rm -rf "$DEST_ASSET_DIR/$asset_type"
        cp -r "$source_dir" "$DEST_ASSET_DIR/"
    fi
done

# NEW: Set correct ownership for all copied assets
echo "  -> Setting ownership for assets..."
chown -R 1000:1000 "$DEST_ASSET_DIR"
echo -e "  -> ${GREEN}Done.${NC}"

echo "--------------------------------------------------"
echo -e "${GREEN}Installation complete! All files have been updated.${NC}"
