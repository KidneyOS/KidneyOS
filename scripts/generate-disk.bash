#!/usr/bin/env bash

# Functions ----------------------------------------------------------------------------------------

# Usage / help
usage() {
  echo "Usage: $0 [-s size] [-f filesystem] [-h help]"
  echo "  -s, --size        Set the size (e.g., 500M, 10G)"
  echo "  -f, --file-system Set the file system (fat16, fat32, exfat, ext2, ext4)"
  echo "  -h, --help        Display this help message"
  exit 1
}

# Partition the disk
partition_disk() {
  echo
  echo "Partitioning $disk_name with $partition and $filesystem..."
  echo "sudo parted $disk_name mklabel $partition"
  sudo parted "$disk_name" mklabel "$partition" || { echo "Failed to create partition label."; exit 1; }

  echo "sudo parted $disk_name mkpart primary 2048s 100%"
  sudo parted "$disk_name" mkpart primary 2048s 100% || { echo "Failed to create partition."; exit 1; }
}

# Format the disk with the given filesystem
format_disk() {
  echo
  echo "Formatting ${loop_device}p1 with $filesystem..."

  case "$filesystem" in
    fat16)
      echo "sudo mkfs.vfat -F 16 ${loop_device}p1"
      sudo mkfs.vfat -F 16 "${loop_device}p1" || { echo "Failed to format FAT16."; return 1; } ;;
    fat32)
      echo "sudo mkfs.vfat -F 32 ${loop_device}p1"
      sudo mkfs.vfat -F 32 "${loop_device}p1" || { echo "Failed to format FAT32."; return 1; } ;;
    exfat)
      echo "sudo mkfs.exfat ${loop_device}p1"
      sudo mkfs.exfat "${loop_device}p1" || { echo "Failed to format ExFAT."; return 1; } ;;
    ext2)
      echo "sudo mkfs.ext2 ${loop_device}p1"
      sudo mkfs.ext2 "${loop_device}p1" || { echo "Failed to format EXT2."; return 1; } ;;
    ext4)
      echo "sudo mkfs.ext4 ${loop_device}p1"
      sudo mkfs.ext4 "${loop_device}p1" || { echo "Failed to format EXT4."; return 1; } ;;
    *)
      echo "Unsupported filesystem: $filesystem"; exit 1 ;;
  esac
}

# Parse arguments ----------------------------------------------------------------------------------

# Disk size
size=""
# Partition type
partition="gpt"
# File system
filesystem=""

while [[ "$#" -gt 0 ]]; do
  case $1 in
  # Size
    -s|--size)
      size="$2"
      shift 2
      ;;
    -f|--file-system)
      filesystem="$2"
      # Ensure filesystem is one of the valid options
      if [[ ! "$filesystem" =~ ^(fat16|fat32|exfat|ext2|ext4)$ ]]; then
        echo "Error: Invalid file system. Allowed values are fat16, fat32, exfat, ext2, ext4."
        usage
      fi
      shift 2
      ;;
    -h|--help)
      usage
      ;;
    *)
      echo "Error: Invalid option $1"
      usage
      ;;
  esac
done

# Check all arguments are provided
if [[ -z "$size" || -z "$filesystem" ]]; then
  echo "Error: All options (-s, -f) are required."
  usage
fi

# Check if the user is in the sudo or wheel group --------------------------------------------------
user=$(whoami)

if [[ "$user" == "root" ]]; then
  echo "Warning: For security reasons, do not run this script as root or with sudo."
  echo "Exiting..."
  exit 1
fi

#if groups "$user" | grep -qE "\bsudo\b|\bwheel\b"; then
#  echo "Warning: This script requires elevated privileges (sudo) and performs disk manipulations."
#  echo "Please review the script carefully to avoid potential data loss or system damage."
#
#  read -r -p "Do you wish to proceed? [y/N]: " response
#  if [[ "$response" != "y" && "$response" != "Y" ]]; then
#    echo "Operation aborted by the user."
#    exit 1
#  fi
#else
#  echo "Error: You do not have the necessary sudo privileges. Please run the script as a user with sufficient permissions."
#  exit 1
#fi

# Create disk image --------------------------------------------------------------------------------

cd "$(dirname "$0")/.." || { echo "Failed to change to parent directory."; exit 1; }

disk_name="${partition}_${filesystem}_${size}.img"
echo "Creating $disk_name at $(pwd)..."

echo "truncate -s $size $disk_name"
truncate -s "$size" "$disk_name" || { echo "Failed to create disk image."; exit 1; }
partition_disk

echo
echo "Attaching $disk_name to loopback device..."

echo "sudo losetup -f"
loop_device=$(sudo losetup -f) || { echo "No available loopback device found."; exit 1; }
echo "losetup $loop_device -P $disk_name"
sudo losetup "$loop_device" -P "$disk_name" || { echo "Failed to attach loopback device $loop_device."; exit 1; }

format_disk
ret=$?

echo
echo "losetup -d $loop_device"
sudo losetup -d "$loop_device" || { echo "Failed to detach loopback device $loop_device."; exit 1; }

if [[ $ret -ne 0 ]]; then
  # failed to format disk
  exit 1
fi

echo
echo "Disk image created: $disk_name"
echo "--------------------------------------------------"
sudo parted "$disk_name" print
