BUILDDIR = build

FLOPPY = boot.img

ifeq ($(VERBOSE), 1)
Q =
else
Q = @
endif

.DEFAULT_GOAL=$(FLOPPY)

MKDOSFS_FLAGS = -n "RLBOOT" -F 12

STAGE2_MAP=build/stage2.map

#ifneq ($(CROSS_COMPILE),)
#	CC      := $(CROSS_COMPILE)gcc
#	AS      := $(CROSS_COMPILE)as
#	LD      := $(CROSS_COMPILE)ld
#	AR      := $(CROSS_COMPILE)ar
#	STRIP   := $(CROSS_COMPILE)strip
#	OBJCOPY := $(CROSS_COMPILE)objcopy
#else
#	OBJCOPY := objcopy
#	STRIP   := strip
#endif

# Currently this is only working with LLVM, due to Rust using LLVM
CC := clang
# Using clang for the first stage causes issues, as it produces 3-byte jmp
# instruction where we want a 2-byte instruction
AS := as
STRIP := strip
OBJCOPY := objcopy

SECTOR_MAPPER := tools/sector_mapper/Cargo.toml

include stage1.mk
include stage2.mk

$(FLOPPY): $(STAGE1) $(STAGE2) $(SECTOR_MAPPER)
	$(Q) rm -f $@.tmp
	$(Q) mkdosfs $(MKDOSFS_FLAGS) -C $@.tmp 1440
	$(Q) tools/rlboot_prepare.sh $@.tmp
	$(Q) mcopy -D oO -i $@.tmp rlboot.cfg.example "::/RLBOOT/RLBOOT.CFG"
# Only update the target if the previous commands succeed
	$(Q) mv $@.tmp $@


$(BUILDDIR):
	$(Q) mkdir -p $@

$(SECTOR_MAPPER):
	$(Q) cargo build --release --manifest-path=$(SECTOR_MAPPER)

emu: $(FLOPPY)
	$(Q) qemu-system-i386 -fda $(FLOPPY) -serial stdio -machine pc -no-reboot

# Create socket for COM1
# NOTE: For certain applications, like XMODEM transfers, direct read/write of
# the socket is too fast, as qemu does not actually respect the 8250's set baud
# rate unless using a physical port. 
emu-sock: $(FLOPPY)
	$(Q) qemu-system-i386 -fda $(FLOPPY) -machine pc -no-reboot                         \
	                      -chardev socket,id=serial0,path=./com1.sock,server=on,debug=9 \
	                      -serial chardev:serial0

# Emulate more realistic floppy disk speeds
emu-slow: $(FLOPPY)
	$(Q) qemu-system-i386 -drive file=$(FLOPPY),if=floppy,format=raw,bps=4000 \
		                  -serial stdio -machine pc -no-reboot

# Enable GDB server
emu-dbg: $(FLOPPY)
	$(Q) qemu-system-i386 -fda $(FLOPPY) -serial stdio -machine pc -no-reboot -S -s

emu-sock-dbg: $(FLOPPY)
	$(Q) qemu-system-i386 -fda $(FLOPPY) -machine pc -no-reboot -S -s           \
	                      -chardev socket,id=serial0,path=./com1.sock,server=on \
	                      -serial chardev:serial0

check: stage2_check

clean: stage1_clean stage2_clean
	$(Q) rm -f $(STAGE1) $(FLOPPY)
	$(Q) cargo clean --release --manifest-path=$(SECTOR_MAPPER)

.PHONY: clean check emu emu-dbg $(SECTOR_MAPPER)
