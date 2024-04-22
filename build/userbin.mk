USER_BIN_RULES := $(addprefix make-userbin-, $(USER_BIN_NAMES))

.PHONY : $(USER_BIN_RULES)
$(USER_BIN_RULES) :
	@$(MAKE) EXTRA_CFLAGS=$(CFLAGS) -s -C $(USER_SRC_ROOT) $(subst make-userbin-,,$@)

$(USER_BINS) : $(USER_SRC_ROOT)/build/% : make-userbin-%