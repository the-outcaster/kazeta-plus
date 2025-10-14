#!/bin/bash

# ---
# Script to automate the creation of a Kazeta+ upgrade kit.
# It prompts for a version number, creates the necessary directory structure,
# and copies all required files from the development environment.
# ---

# Exit immediately if a command exits with a non-zero status.
set -e

# --- Configuration ---
# Set the source directory for your Kazeta+ project files.
SOURCE_DIR="$HOME/Programs/kazeta-plus"
# Set the destination directory where the kit will be created.
DEST_BASE_DIR="$HOME/Desktop/kazeta_assets/upgrade_kits/"


# --- Main Logic ---

# 1. Prompt for the version number
read -p "Enter the version number for the new upgrade kit (e.g., 1.11): " VERSION

if [ -z "$VERSION" ]; then
    echo "Error: Version number cannot be empty."
    exit 1
fi

# 2. Define kit directory and paths
KIT_DIR_NAME="kazeta-plus-upgrade-kit-$VERSION"
KIT_FULL_PATH="$DEST_BASE_DIR/$KIT_DIR_NAME"

echo "-----------------------------------------------------"
echo "Creating Kazeta+ Upgrade Kit v$VERSION"
echo "Source: $SOURCE_DIR"
echo "Destination: $KIT_FULL_PATH"
echo "-----------------------------------------------------"

# Check if the destination directory already exists
if [ -d "$KIT_FULL_PATH" ]; then
    read -p "Directory '$KIT_FULL_PATH' already exists. Overwrite? (y/n): " CONFIRM
    if [[ "$CONFIRM" != "y" ]]; then
        echo "Aborted."
        exit 0
    fi
    echo "Removing existing directory..."
    rm -rf "$KIT_FULL_PATH"
fi

# 3. Create the directory structure
echo "Creating directory structure..."
mkdir -p "$KIT_FULL_PATH/rootfs/etc/keyd"
mkdir -p "$KIT_FULL_PATH/rootfs/etc/sudoers.d"
mkdir -p "$KIT_FULL_PATH/rootfs/etc/systemd/system"
mkdir -p "$KIT_FULL_PATH/rootfs/usr/bin"
mkdir -p "$KIT_FULL_PATH/rootfs/usr/share/inputplumber/profiles"
echo "Directory structure created."

# 4. Download the main upgrade script
echo "Downloading upgrade-to-plus.sh script..."
curl -sL "https://raw.githubusercontent.com/the-outcaster/kazeta-plus/main/upgrade-to-plus.sh" \
     -o "$KIT_FULL_PATH/upgrade-to-plus.sh"
# Make the script executable
chmod +x "$KIT_FULL_PATH/upgrade-to-plus.sh"
echo "Download complete."

# 5. Copy all necessary files
echo "Copying files from rootfs..."

# etc files
cp "$SOURCE_DIR/rootfs/etc/keyd/default.conf" "$KIT_FULL_PATH/rootfs/etc/keyd/"
cp "$SOURCE_DIR/rootfs/etc/sudoers.d/99-kazeta-plus" "$KIT_FULL_PATH/rootfs/etc/sudoers.d/"
cp "$SOURCE_DIR/rootfs/etc/systemd/system/kazeta-profile-loader.service" "$KIT_FULL_PATH/rootfs/etc/systemd/system/"

# usr/bin shell scripts
echo "Copying shell scripts..."
scripts_to_copy=(
    "ethernet-connect"
    "kazeta"
    "kazeta-copy-logs"
    "kazeta-mount"
    "kazeta-session"
    "kazeta-wifi-setup"
)
for script in "${scripts_to_copy[@]}"; do
    cp "$SOURCE_DIR/rootfs/usr/bin/$script" "$KIT_FULL_PATH/rootfs/usr/bin/"
done

# usr/bin bios binary
echo "Copying kazeta-bios binary..."
cp "$SOURCE_DIR/bios/target/debug/kazeta-bios" "$KIT_FULL_PATH/rootfs/usr/bin/"

# usr/share files
echo "Copying inputplumber profile..."
cp "$SOURCE_DIR/rootfs/usr/share/inputplumber/profiles/steam-deck.yaml" "$KIT_FULL_PATH/rootfs/usr/share/inputplumber/profiles/"

echo "All files copied successfully."
echo "-----------------------------------------------------"
echo "Success! Upgrade kit created at:"
echo "$KIT_FULL_PATH"
echo "-----------------------------------------------------"
