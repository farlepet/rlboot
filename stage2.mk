# Stage 2 bootloader makefile

S2_BUILDDIR = $(BUILDDIR)/stage2

STAGE2      = $(S2_BUILDDIR)/stage2.bin

CARGO ?= cargo
CARGO_TARGET = i386-unknown-none
CARGO_RELEASE = release

S2_SRCDIR = src
S2_INCDIR = inc

S2_LDFLAGS = -melf_i386
S2_CFLAGS  = -fno-pic -I $(S2_INCDIR) \
			 -nostdlib -nostdinc -ffreestanding -Wall -Wextra -Werror -Os \
			 -g -fno-stack-protector -fdata-sections -ffunction-sections \
			 -Wl,--gc-sections
			 -include "inc/config.h"
S2_ASFLAGS = $(S2_CFLAGS)

ifeq ($(CC), clang)
	S2_CFLAGS += -Wno-unused-command-line-argument \
				 --target=i386-unknown-none
else
	S2_CFLAGS += -m32 -march=i386
endif

# TODO: Allow file selection based on features, possible via cmake
S2_SRCS = $(S2_SRCDIR)/startup/startup.s           #\
#		  $(S2_SRCDIR)/bios/bios_asm.s             \
#		  $(S2_SRCDIR)/intr/int_wrappers.s         \

S2_OBJS = $(filter %.o,$(patsubst $(S2_SRCDIR)/%.c,$(S2_BUILDDIR)/%.o,$(S2_SRCS)) \
                       $(patsubst $(S2_SRCDIR)/%.s,$(S2_BUILDDIR)/%.o,$(S2_SRCS)))
S2_DEPS = $(filter %.d,$(patsubst $(S2_SRCDIR)/%.c,$(S2_BUILDDIR)/%.d,$(S2_SRCS)))

S2_RUST_OBJ = target/$(CARGO_TARGET)/$(CARGO_RELEASE)/librlboot.a

$(STAGE2): $(S2_OBJS) $(S2_RUST_OBJ)
	@echo -e "\033[32m    \033[1mLD\033[21m    \033[34m$@\033[0m"
	$(Q) $(LD) $(S2_LDFLAGS) -r -o $(STAGE2).o $(S2_OBJS) $(S2_RUST_OBJ)
	$(Q) $(CC) $(S2_CFLAGS) -o $(STAGE2).elf $(STAGE2).o -T stage2.ld -nostdlib -lgcc -latomic
	$(Q) $(OBJCOPY) -O binary --only-section=.text --only-section=.rodata --only-section=.data $(STAGE2).elf $@

$(S2_RUST_OBJ):
	$(Q) $(CARGO) build -Z build-std="core,alloc" --target=$(CARGO_TARGET).json --$(CARGO_RELEASE)

#$(S2_BUILDDIR)/%.o: $(S2_SRCDIR)/%.c
#	@echo -e "\033[32m    \033[1mCC\033[21m    \033[34m$<\033[0m"
#	$(Q) mkdir -p $(dir $@)
#	$(Q) $(CC) $(S2_CFLAGS) -MMD -MP --save-temps -c -o $@ $<



$(S2_BUILDDIR)/%.o: $(S2_SRCDIR)/%.s
	@echo -e "\033[32m    \033[1mAS\033[21m    \033[34m$<\033[0m"
	$(Q) mkdir -p $(dir $@)
	$(Q) $(CC) $(S2_CFLAGS) -MMD -MP -c -o $@ $<




stage2_clean:
	$(Q) rm -f $(STAGE2) $(STAGE2).elf $(S2_OBJS) $(S2_DEPS)
	$(Q) $(CARGO) clean --release

.PHONY: stage2_clean $(S2_RUST_OBJ)

-include $(S2_DEPS)

