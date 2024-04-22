
KERNEL_MODULE_LIBS := $(addprefix lib,$(KERNEL_MODULE_NAMES))
KERNEL_MODULE_LIBS := $(addsuffix .a,$(KERNEL_MODULE_LIBS))
KERNEL_MODULE_LIBS := $(addprefix $(TARGET_ROOT)/,$(KERNEL_MODULE_LIBS))

$(KERNEL_MODULE_LIBS) : $(TARGET_ROOT)/lib%.a : $(LIB_KERNEL) cargo-buildlib-% 

$(KERNEL_MODULES) : $(TARGET_ROOT)/%.ko : $(TARGET_ROOT)/lib%.a
	@echo LD $(patsubst %.ko,lib%.a,$(notdir $@))
	@$(LD) $(LDFLAG)		\
		--whole-archive		\
		-R $(KERNEL_BIN)	\
		-r					\
		-o $@				\
		$<
	@echo OBJCOPY $(notdir $@)
	@$(OBJCOPY) --strip-debug $@