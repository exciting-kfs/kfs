# tools 

OS := $(shell uname -s)
ifeq ($(OS), Linux)
PREFIX := i686-linux-gnu-
CFLAGS := -fno-pie -fno-stack-protector # for removing stack canary.
LINKER_SCRIPT = $(realpath .)/userspace.ld
LDFLAGS := -n --script=$(LINKER_SCRIPT) --oformat=binary -nostdlib
else
PREFIX := i686-elf-
CFLAGS =
LDFLAGS := -n --oformat=binary -nostdlib
endif

CC := $(PREFIX)gcc
export CC

AR := $(PREFIX)ar
export AR

AS := nasm
export AS

LD := $(PREFIX)ld
export LD

# common flags

CFLAGS += -Wall -Wextra -nostdlib
CFLAGS += -I$(realpath include)
export CFLAGS

BUILDDIR := $(realpath .)/build
export BUILDDIR

COMMON := $(realpath common.mk)
export COMMON

export LDFLAGS

# targets

MODULES := init shell fortytwo test_sig test_setXid test_sig_stop_cont

LIBSYSTEM := $(BUILDDIR)/libsystem.a

# recipes

.PHONY : all
all : $(MODULES)

.PHONY : $(MODULES)
$(MODULES) : $(LIBSYSTEM) | $(BUILDDIR)
	@echo BUILD $@.bin
	@$(MAKE) -s -C $@

.PHONY : $(LIBSYSTEM)
$(LIBSYSTEM) : | $(BUILDDIR)
	@echo BUILD libsystem.a
	@$(MAKE) -s -C libsystem $@ 

$(BUILDDIR) :
	@mkdir $@

.PHONY : clean
clean :
	@rm -rf $(BUILDDIR) 

.PHONY : re
re : clean
	@$(MAKE) all
