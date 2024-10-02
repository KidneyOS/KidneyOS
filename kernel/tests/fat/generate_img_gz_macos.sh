#!/bin/sh

DIR="$1"

if [ "$#" -ne 1 ] || [ '!' -d "$DIR" ]; then
    echo "Usage $0 <directory name>"
    exit 1
fi

echo "$DIR" | grep -q '/' && { echo "Directory name must not contain /"; exit 1; }

if [ -e 'mnt' ]; then
    rmdir mnt || { echo "Remove ./mnt before running this."; exit 1; }
fi

copy_filesystem16() {
    rm -f "$1".gz
    echo '    Mounting the image...'
    mkdir -p mnt || exit 1

    # Use hdiutil instead of mount
    hdiutil attach -imagekey diskimage-class=CRawDiskImage -nomount "$1" || exit 1
    DEV=$(hdiutil info | grep '/dev/disk' | tail -n 1 | awk '{print $1}')
    sudo newfs_msdos -F 16 $DEV || exit 1
    sudo mount -t msdos $DEV mnt || exit 1

    echo '    Copying the files to the image...'
    sudo cp -r "${DIR}"/* mnt/ || exit 1

    # Set modify time of all files to get consistent image (using macOS touch)
    sudo find mnt -type f -exec touch -t 202001011234.50 '{}' ';' || exit 1

    echo '    Unmounting the image...'
    sudo umount mnt || exit 1
    hdiutil detach $DEV || exit 1

    echo '    Compressing the image...'
    gzip "$1" || exit 1
}

copy_filesystem32() {
    rm -f "$1".gz
    echo '    Mounting the image...'
    mkdir -p mnt || exit 1

    # Use hdiutil instead of mount
    hdiutil attach -imagekey diskimage-class=CRawDiskImage -nomount "$1" || exit 1
    DEV=$(hdiutil info | grep '/dev/disk' | tail -n 1 | awk '{print $1}')
    sudo newfs_msdos -F 32 $DEV || exit 1
    sudo mount -t msdos $DEV mnt || exit 1

    echo '    Copying the files to the image...'
    sudo cp -r "${DIR}"/* mnt/ || exit 1

    # Set modify time of all files to get consistent image (using macOS touch)
    sudo find mnt -type f -exec touch -t 202001011234.50 '{}' ';' || exit 1

    echo '    Unmounting the image...'
    sudo umount mnt || exit 1
    hdiutil detach $DEV || exit 1

    echo '    Compressing the image...'
    gzip "$1" || exit 1
}

# 35M is around the minimum size of a FAT-32 filesystem with 512-byte clusters
echo 'Creating FAT-32 image'
rm -f "${DIR}_fat32.img"
echo dd if=/dev/zero of="${DIR}_fat32.img" bs=1024 count=35000
dd if=/dev/zero of="${DIR}_fat32.img" bs=1024 count=35000 || exit 1
mkfs.vfat -F 32 -s 1 -S 512 "${DIR}_fat32.img" || exit 1
copy_filesystem32 "${DIR}_fat32.img"

# Create FAT-16 image
echo 'Creating FAT-16 image'
rm -f "${DIR}_fat16.img"
dd if=/dev/zero of="${DIR}_fat16.img" bs=1024 count=16384 || exit 1
mkfs.vfat -F 16 -s 1 -S 512 "${DIR}_fat16.img" || exit 1
copy_filesystem16 "${DIR}_fat16.img"
