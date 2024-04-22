ifeq ($(HDD_FAST_BUILD),y)
HDD_SCRIPT := scripts/hdd/make-hdd-fast.sh
else
HDD_SCRIPT := scripts/hdd/make-hdd.sh
endif

$(TARGET_ROOT)/sysroot : $(USER_BINS) $(KERNEL_MODULES) scripts/hdd/make-sysroot.sh
	@echo MAKE sysroot
	@rm -rf $(TARGET_ROOT)/sysroot
	@mkdir -p $(TARGET_ROOT)/sysroot
	@scripts/hdd/make-sysroot.sh $(TARGET_ROOT)/sysroot
	@cp $(KERNEL_MODULES) $(TARGET_ROOT)/sysroot/lib/modules
	@cp $(USER_BINS) $(TARGET_ROOT)/sysroot/bin

$(HDD_IMG) : $(USER_BINS) $(TARGET_ROOT)/sysroot $(HDD_SCRIPT)
	@echo MAKE $(notdir $@)
	$(HDD_SCRIPT) $@ $(TARGET_ROOT)/sysroot
