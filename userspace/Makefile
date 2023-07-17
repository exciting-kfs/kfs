PREFIX = i686-elf-

GCC := $(PREFIX)gcc
AR := $(PREFIX)ar

CFLAGS := -Wall -Wextra -Werror -nostdlib
CFLAGS += -Iinclude

BUILDDIR := build

LIBSYSTEM := $(BUILDDIR)/libsystem.a

.PHONY : all
all : $(BUILDDIR)/forkbomb.bin

$(BUILDDIR) :
	mkdir $(BUILDDIR)

$(LIBSYSTEM) : $(addprefix $(BUILDDIR)/,crt0.o syscall.o) | $(BUILDDIR)
	$(AR) -rcs $@ --output $(BUILDDIR) $^

$(BUILDDIR)/forkbomb.bin : $(LIBSYSTEM) $(BUILDDIR)/forkbomb.o | $(BUILDDIR)
	$(GCC) $(CFLAGS) -Wl,--oformat=binary -o $@ $^

$(BUILDDIR)/%.o : %.c | $(BUILDDIR)
	$(GCC) $(CFLAGS) -c $^ -o $@

$(BUILDDIR)/%.o : %.S | $(BUILDDIR)
	nasm -felf -o $@ $^

.PHONY : clean
clean :
	rm -rf $(BUILDDIR) 

.PHONY : re
re : clean
	$(MAKE) all
