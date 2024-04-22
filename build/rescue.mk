GRUB2_MKRESCUE=$(I386_GRUB2_PREFIX)/bin/grub-mkrescue
GRUB2_I386_LIB=$(I386_GRUB2_PREFIX)/lib/grub/i386-pc

RESUCE_SRC_ROOT := iso
RESCUE_TARGET_ROOT := $(TARGET_ROOT)/iso

$(RESCUE_IMG): kernel $(shell find $(RESUCE_SRC_ROOT) -type f) $(KERNEL_DEBUG_SYMBOL)
	@echo MKRESCUE $(notdir $@)
	@mkdir -p $(TARGET_ROOT)/boot
	@cp -r $(RESUCE_SRC_ROOT) $(TARGET_ROOT)
	@cp $(KERNEL_BIN) $(RESCUE_TARGET_ROOT)/boot
	@$(GRUB2_MKRESCUE) -d $(GRUB2_I386_LIB) $(RESCUE_TARGET_ROOT) -o $@ 2>/dev/null >/dev/null
