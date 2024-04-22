LINKER_SCRIPT := linker-script/kernel.ld

KERNEL_ELF_NAME := $(KERNEL_BIN_NAME).elf
KERNEL_ELF := $(TARGET_ROOT)/$(KERNEL_ELF_NAME)

$(LIB_KERNEL) : make-userbin-init
	@$(MAKE) cargo-buildlib-kfs

$(KERNEL_ELF) : $(LIB_KERNEL) $(LINKER_SCRIPT)
	@echo LD $(notdir $@)
	@$(LD) $(LDFLAG)		\
		--whole-archive		\
		-T $(LINKER_SCRIPT) \
		-o $@				\
		$(LIB_KERNEL)

$(KERNEL_BIN) : $(KERNEL_ELF)
	@echo OBJCOPY $(notdir $@)
	@$(OBJCOPY) --strip-debug $< $(KERNEL_BIN)

$(KERNEL_DEBUG_SYMBOL) : $(KERNEL_ELF)
	@echo OBJCOPY $(notdir $@)
	@$(OBJCOPY) --only-keep-debug $< $(KERNEL_DEBUG_SYMBOL)