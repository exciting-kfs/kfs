# tools 

OS := $(shell uname -s)
ifeq ($(OS), Linux)
PREFIX := i686-linux-gnu-
else
PREFIX := i686-elf-
endif

CC := $(PREFIX)gcc
export CC

AR := $(PREFIX)ar
export AR

AS := nasm
export AS

# common flags

CFLAGS := -Wall -Wextra -Werror -nostdlib
CFLAGS += -I$(realpath include)
export CFLAGS

BUILDDIR := $(realpath .)/build
export BUILDDIR

COMMON := $(realpath common.mk)
export COMMON

# targets

MODULES := init shell fortytwo

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
