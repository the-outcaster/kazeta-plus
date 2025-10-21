#!/bin/bash

# ---
# Script to automate the creation of a Kazeta+ upgrade kit.
# It checks for debug flags, creates the directory structure, copies all
# required files, and zips the final kit for release.
# ---

# Exit immediately if a command exits with a non-zero status.
set -e

# --- Configuration ---
SOURCE_DIR="$HOME/Programs/kazeta-plus"
DEST_BASE_DIR="$HOME/Desktop/kazeta_assets/upgrade_kits"
MAIN_RS_PATH="$SOURCE_DIR/bios/src/main.rs"

# --- Pre-flight Checks ---
echo "Performing pre-flight checks..."
if grep -q "const DEBUG_GAME_LAUNCH: bool = true;" "$MAIN_RS_PATH" || grep -q "const DEV_MODE: bool = true;" "$MAIN_RS_PATH"; then
    echo "-----------------------------------------------------"
    echo "ERROR: A debug flag is set to 'true' in main.rs."
    echo "Please set DEBUG_GAME_LAUNCH and DEV_MODE to 'false' before creating a release kit."
    echo "-----------------------------------------------------"
    exit 1
fi
echo "Checks passed. Proceeding with kit creation."
echo "-----------------------------------------------------"

# --- Main Logic ---
read -p "Enter the version number for the new upgrade kit (e.g., 1.2): " VERSION
if [ -z "$VERSION" ]; then
    echo "Error: Version number cannot be empty."
    exit 1
fi

KIT_DIR_NAME="kazeta-plus-upgrade-kit-$VERSION"
KIT_FULL_PATH="$DEST_BASE_DIR/$KIT_DIR_NAME"

echo "Creating Kazeta+ Upgrade Kit v$VERSION"
echo "Source: $SOURCE_DIR"
echo "Destination: $KIT_FULL_PATH"
echo "-----------------------------------------------------"

if [ -d "$KIT_FULL_PATH" ]; then
    read -p "Directory '$KIT_FULL_PATH' already exists. Overwrite? (y/n): " CONFIRM
    if [[ "$CONFIRM" != "y" ]]; then
        echo "Aborted."
        exit 0
    fi
    echo "Removing existing directory..."
    rm -rf "$KIT_FULL_PATH"
fi

# 4. Create the directory structure
echo "Creating directory structure..."
mkdir -p "$KIT_FULL_PATH/rootfs/etc/keyd"
mkdir -p "$KIT_FULL_PATH/rootfs/etc/sudoers.d"
mkdir -p "$KIT_FULL_PATH/rootfs/etc/systemd/system"
mkdir -p "$KIT_FULL_PATH/rootfs/etc/udev/rules.d"
mkdir -p "$KIT_FULL_PATH/rootfs/usr/bin"
mkdir -p "$KIT_FULL_PATH/rootfs/usr/share/inputplumber/profiles"
mkdir -p "$KIT_FULL_PATH/aur-pkgs"
echo "Directory structure created."

# 5. Download the main upgrade script
echo "Downloading upgrade-to-plus.sh script..."
curl -sL "https://raw.githubusercontent.com/the-outcaster/kazeta-plus/main/upgrade-to-plus.sh" \
     -o "$KIT_FULL_PATH/upgrade-to-plus.sh"
chmod +x "$KIT_FULL_PATH/upgrade-to-plus.sh"
echo "Download complete."

# 6. Copy all necessary files from your local dev environment
echo "Copying files from rootfs..."
cp "$SOURCE_DIR/rootfs/etc/keyd/default.conf" "$KIT_FULL_PATH/rootfs/etc/keyd/"
cp "$SOURCE_DIR/rootfs/etc/sudoers.d/99-kazeta-plus" "$KIT_FULL_PATH/rootfs/etc/sudoers.d/"
cp "$SOURCE_DIR/rootfs/etc/systemd/system/kazeta-profile-loader.service" "$KIT_FULL_PATH/rootfs/etc/systemd/system/"
cp "$SOURCE_DIR/rootfs/etc/udev/rules.d/51-gcadapter.rules" "$KIT_FULL_PATH/rootfs/etc/udev/rules.d/"

echo "Copying shell scripts..."
scripts_to_copy=( "ethernet-connect" "kazeta" "kazeta-copy-logs" "kazeta-mount" "kazeta-session" "kazeta-wifi-setup" )
for script in "${scripts_to_copy[@]}"; do
    cp "$SOURCE_DIR/rootfs/usr/bin/$script" "$KIT_FULL_PATH/rootfs/usr/bin/"
done

echo "Copying kazeta-bios binary..."
# --- RECOMMENDED CHANGE: Copy release binary ---
# You usually want the release build in the kit, not debug
if [ -f "$SOURCE_DIR/bios/target/release/kazeta-bios" ]; then
    cp "$SOURCE_DIR/bios/target/release/kazeta-bios" "$KIT_FULL_PATH/rootfs/usr/bin/"
else
    echo "WARNING: Release binary not found, copying debug binary."
    cp "$SOURCE_DIR/bios/target/debug/kazeta-bios" "$KIT_FULL_PATH/rootfs/usr/bin/"
fi

echo "Copying inputplumber profile..."
cp "$SOURCE_DIR/rootfs/usr/share/inputplumber/profiles/steam-deck.yaml" "$KIT_FULL_PATH/rootfs/usr/share/inputplumber/profiles/"

echo "Copying gcadapter-oc-dkms source..."
cp -r "$SOURCE_DIR/aur-pkgs/gcadapter-oc-dkms" "$KIT_FULL_PATH/aur-pkgs/"

echo "All files copied successfully."

# 7. Create the ZIP archive
echo "Creating ZIP archive..."
(
    # Go one level up from the kit directory to include the base folder in the zip
    cd "$DEST_BASE_DIR" && \
    zip -r "$KIT_DIR_NAME.zip" "$KIT_DIR_NAME"
)
echo "ZIP archive created."

echo "-----------------------------------------------------"
echo "Success! Upgrade kit created at:"
echo "$KIT_FULL_PATH"
echo "and"
echo "$DEST_BASE_DIR/$KIT_DIR_NAME.zip" # Corrected zip path display
echo "-----------------------------------------------------"
echo "Reminder: Manually create and upload 'kazeta-wifi-pack.zip' to the release page."
echo "Users will need to place the unzipped 'kazeta-wifi-pack' folder next to the upgrade script."
echo "-----------------------------------------------------"
