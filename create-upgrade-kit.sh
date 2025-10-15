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
DEST_BASE_DIR="$HOME/Desktop/kazeta_assets/upgrade_kits"
# Base URL for downloading the local packages from your GitHub repo.
PACKAGES_BASE_URL="https://github.com/the-outcaster/kazeta-plus/raw/main/local_packages"


# --- Main Logic ---

# 1. Prompt for the version number
read -p "Enter the version number for the new upgrade kit (e.g., 1.2): " VERSION

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
mkdir -p "$KIT_FULL_PATH/local_packages"
echo "Directory structure created."

# 4. Download the main upgrade script
echo "Downloading upgrade-to-plus.sh script..."
curl -sL "https://raw.githubusercontent.com/the-outcaster/kazeta-plus/main/upgrade-to-plus.sh" \
     -o "$KIT_FULL_PATH/upgrade-to-plus.sh"
chmod +x "$KIT_FULL_PATH/upgrade-to-plus.sh"
echo "Download complete."

# 5. Download the complete set of local Wi-Fi packages and dependencies
echo "Downloading local Wi-Fi packages..."
packages_to_download=(
    "acl-2.3.2-1-x86_64.pkg.tar.zst"
    "attr-2.5.2-1-x86_64.pkg.tar.zst"
    "audit-4.0.5-1-x86_64.pkg.tar.zst"
    "bash-5.3.3-2-x86_64.pkg.tar.zst"
    "brotli-1.1.0-3-x86_64.pkg.tar.zst"
    "bzip2-1.0.8-6-x86_64.pkg.tar.zst"
    "ca-certificates-20240618-1-any.pkg.tar.zst"
    "ca-certificates-mozilla-3.115-1-x86_64.pkg.tar.zst"
    "ca-certificates-utils-20240618-1-any.pkg.tar.zst"
    "coreutils-9.7-1-x86_64.pkg.tar.zst"
    "curl-8.15.0-1-x86_64.pkg.tar.zst"
    "dbus-1.16.2-1-x86_64.pkg.tar.zst"
    "duktape-2.7.0-7-x86_64.pkg.tar.zst"
    "e2fsprogs-1.47.3-1-x86_64.pkg.tar.zst"
    "ell-0.78-1-x86_64.pkg.tar.zst"
    "expat-2.7.1-1-x86_64.pkg.tar.zst"
    "file-5.46-5-x86_64.pkg.tar.zst"
    "filesystem-2025.05.03-1-any.pkg.tar.zst"
    "findutils-4.10.0-3-x86_64.pkg.tar.zst"
    "gcc-libs-15.2.1+r22+gc4e96a094636-1-x86_64.pkg.tar.zst"
    "gdbm-1.25-1-x86_64.pkg.tar.zst"
    "glib2-2.84.4-2-x86_64.pkg.tar.zst"
    "glibc-2.42+r17+gd7274d718e6f-1-x86_64.pkg.tar.zst"
    "gmp-6.3.0-2-x86_64.pkg.tar.zst"
    "gnutls-3.8.10-1-x86_64.pkg.tar.zst"
    "gpm-1.20.7.r38.ge82d1a6-6-x86_64.pkg.tar.zst"
    "iana-etc-20250612-1-any.pkg.tar.zst"
    "iproute2-6.16.0-2-x86_64.pkg.tar.zst"
    "iptables-nft-1:1.8.11-2-x86_64.pkg.tar.zst"
    "iwd-3.9-1-x86_64.pkg.tar.zst"
    "jansson-2.14.1-1-x86_64.pkg.tar.zst"
    "json-c-0.18-2-x86_64.pkg.tar.zst"
    "keyutils-1.6.3-3-x86_64.pkg.tar.zst"
    "krb5-1.21.3-2-x86_64.pkg.tar.zst"
    "leancrypto-1.5.1-1-x86_64.pkg.tar.zst"
    "libbpf-1.5.1-1-x86_64.pkg.tar.zst"
    "libcap-2.76-1-x86_64.pkg.tar.zst"
    "libcap-ng-0.8.5-3-x86_64.pkg.tar.zst"
    "libdaemon-0.14-6-x86_64.pkg.tar.zst"
    "libelf-0.193-5-x86_64.pkg.tar.zst"
    "libevent-2.1.12-4-x86_64.pkg.tar.zst"
    "libffi-3.5.1-1-x86_64.pkg.tar.zst"
    "libgcrypt-1.11.1-1-x86_64.pkg.tar.zst"
    "libgpg-error-1.55-1-x86_64.pkg.tar.zst"
    "libidn2-2.3.7-1-x86_64.pkg.tar.zst"
    "libldap-2.6.10-2-x86_64.pkg.tar.zst"
    "libmm-glib-1.24.2-1-x86_64.pkg.tar.zst"
    "libmnl-1.0.5-2-x86_64.pkg.tar.zst"
    "libndp-1.9-1-x86_64.pkg.tar.zst"
    "libnetfilter_conntrack-1.0.9-2-x86_64.pkg.tar.zst"
    "libnewt-0.52.25-1-x86_64.pkg.tar.zst"
    "libnfnetlink-1.0.2-2-x86_64.pkg.tar.zst"
    "libnftnl-1.3.0-1-x86_64.pkg.tar.zst"
    "libnghttp2-1.66.0-1-x86_64.pkg.tar.zst"
    "libnghttp3-1.11.0-1-x86_64.pkg.tar.zst"
    "libnl-3.11.0-1-x86_64.pkg.tar.zst"
    "libnm-1.54.0-1-x86_64.pkg.tar.zst"
    "libnsl-2.0.1-1-x86_64.pkg.tar.zst"
    "libnvme-1.15-1-x86_64.pkg.tar.zst"
    "libp11-kit-0.25.5-1-x86_64.pkg.tar.zst"
    "libpcap-1.10.5-3-x86_64.pkg.tar.zst"
    "libpgm-5.3.128-3-x86_64.pkg.tar.zst"
    "libpsl-0.21.5-2-x86_64.pkg.tar.zst"
    "libsasl-2.1.28-5-x86_64.pkg.tar.zst"
    "libseccomp-2.5.6-1-x86_64.pkg.tar.zst"
    "libsodium-1.0.20-1-x86_64.pkg.tar.zst"
    "libssh2-1.11.1-1-x86_64.pkg.tar.zst"
    "libsysprof-capture-48.0-7-x86_64.pkg.tar.zst"
    "libtasn1-4.20.0-1-x86_64.pkg.tar.zst"
    "libteam-1.32-2-x86_64.pkg.tar.zst"
    "libtirpc-1.3.6-2-x86_64.pkg.tar.zst"
    "libunistring-1.3-1-x86_64.pkg.tar.zst"
    "liburing-2.11-1-x86_64.pkg.tar.zst"
    "libverto-0.3.2-5-x86_64.pkg.tar.zst"
    "libxcrypt-4.4.38-1-x86_64.pkg.tar.zst"
    "linux-api-headers-6.16-1-x86_64.pkg.tar.zst"
    "linux-firmware-20250808-1-any.pkg.tar.zst"
    "linux-firmware-amdgpu-20250808-1-any.pkg.tar.zst"
    "linux-firmware-atheros-20250808-1-any.pkg.tar.zst"
    "linux-firmware-broadcom-20250808-1-any.pkg.tar.zst"
    "linux-firmware-cirrus-20250808-1-any.pkg.tar.zst"
    "linux-firmware-intel-20250808-1-any.pkg.tar.zst"
    "linux-firmware-mediatek-20250808-1-any.pkg.tar.zst"
    "linux-firmware-nvidia-20250808-1-any.pkg.tar.zst"
    "linux-firmware-other-20250808-1-any.pkg.tar.zst"
    "linux-firmware-radeon-20250808-1-any.pkg.tar.zst"
    "linux-firmware-realtek-20250808-1-any.pkg.tar.zst"
    "linux-firmware-whence-20250808-1-any.pkg.tar.zst"
    "lmdb-0.9.33-1-x86_64.pkg.tar.zst"
    "lz4-1:1.10.0-2-x86_64.pkg.tar.zst"
    "mobile-broadband-provider-info-20250613-1-any.pkg.tar.zst"
    "nettle-3.10.2-1-x86_64.pkg.tar.zst"
    "networkmanager-1.54.0-1-x86_64.pkg.tar.zst"
    "nftables-1:1.1.4-1-x86_64.pkg.tar.zst"
    "nspr-4.37-1-x86_64.pkg.tar.zst"
    "nss-3.115-1-x86_64.pkg.tar.zst"
    "openssl-3.5.2-1-x86_64.pkg.tar.zst"
    "p11-kit-0.25.5-1-x86_64.pkg.tar.zst"
    "pam-1.7.1-1-x86_64.pkg.tar.zst"
    "pambase-20250719-1-any.pkg.tar.zst"
    "pcre-8.45-4-x86_64.pkg.tar.zst"
    "pcre2-10.45-1-x86_64.pkg.tar.zst"
    "pcsclite-2.3.3-1-x86_64.pkg.tar.zst"
    "polkit-126-2-x86_64.pkg.tar.zst"
    "popt-1.19-2-x86_64.pkg.tar.zst"
    "procps-ng-4.0.5-3-x86_64.pkg.tar.zst"
    "readline-8.3.001-1-x86_64.pkg.tar.zst"
    "shadow-4.18.0-1-x86_64.pkg.tar.zst"
    "slang-2.3.3-4-x86_64.pkg.tar.zst"
    "sqlite-3.50.4-1-x86_64.pkg.tar.zst"
    "systemd-libs-257.8-2-x86_64.pkg.tar.zst"
    "tzdata-2025b-1-x86_64.pkg.tar.zst"
    "util-linux-2.41.1-1-x86_64.pkg.tar.zst"
    "util-linux-libs-2.41.1-1-x86_64.pkg.tar.zst"
    "wpa_supplicant-2:2.11-3-x86_64.pkg.tar.zst"
    "xz-5.8.1-1-x86_64.pkg.tar.zst"
    "zeromq-4.3.5-2-x86_64.pkg.tar.zst"
    "zlib-1:1.3.1-2-x86_64.pkg.tar.zst"
    "zstd-1.5.7-2-x86_64.pkg.tar.zst"
)

for pkg in "${packages_to_download[@]}"; do
    echo "  -> Downloading $pkg..."
    # Use -f to fail silently if a file doesn't exist (useful for older/renamed deps)
    curl -fsSL "$PACKAGES_BASE_URL/$pkg" -o "$KIT_FULL_PATH/local_packages/$pkg" || echo "    -> Warning: Could not download $pkg. It may not be required."
done
echo "Local packages downloaded."

# 6. Copy all necessary files from your local dev environment
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
