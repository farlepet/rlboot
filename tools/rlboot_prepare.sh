#!/bin/sh
# RLBoot disk/image preparation script
# Usage:
#   rlboot_prepare.sh <floppy disk device or image file>
#
# Presently must be used within the root of the repository. Assumes required
# tools are already built.

if [[ $# -ne 1 ]]; then
    echo "USAGE:"
    echo "  rlboot_prepare.sh <floppy disk device or image file>"
    exit 2
fi

# Auto-fail
set -e

FLOPPY=$1

RLBOOT_DIR=.
STAGE1=$RLBOOT_DIR/build/stage1/stage1.bin
STAGE2=$RLBOOT_DIR/build/stage2/stage2.bin
STAGE2_MAP=$RLBOOT_DIR/build/stage2.map

SECTOR_MAPPER=$RLBOOT_DIR/tools/sector_mapper/Cargo.toml

if   [[ ! -f $FLOPPY ]]; then
    echo "Floppy/image '$FLOPPY' does not exist!"
    exit 2
elif [[ ! -f $STAGE1 ]]; then
    echo "Stage 1 binary '$STAGE1' does not exist!"
    exit 1
elif [[ ! -f $STAGE2 ]]; then
    echo "Stage 2 binary '$STAGE2' does not exist!"
    exit 1
fi

# Stage 1 / bootsector
dd if=$STAGE1 of=$FLOPPY conv=notrunc iflag=count_bytes,skip_bytes oflag=seek_bytes count=11
dd if=$STAGE1 of=$FLOPPY conv=notrunc iflag=count_bytes,skip_bytes oflag=seek_bytes skip=90 seek=90

# Stage 2 + Map
mmd   -D oO -i $FLOPPY             ::/RLBOOT
mcopy -D oO -i $FLOPPY $STAGE2     ::/RLBOOT/STAGE2.BIN
cargo run -q --release --manifest-path=$SECTOR_MAPPER 1 $FLOPPY "RLBOOT/STAGE2.BIN" $STAGE2_MAP
mcopy -D oO -i $FLOPPY $STAGE2_MAP ::/RLBOOT/STAGE2.MAP
cargo run -q --release --manifest-path=$SECTOR_MAPPER 2 $FLOPPY "RLBOOT/STAGE2.MAP"

# For easier testing
if [[ -f KERNEL ]]; then
    mcopy -D oO -i $FLOPPY KERNEL ::/KERNEL
fi
