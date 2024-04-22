
# RUSTC_FLAG += --cfg log_level='"$(LOG_LEVEL)"' 

ifeq ($(RELEASE_MODE),y)
CARGO_FLAG :=  --release
endif

CARGO_TARGETS := $(addprefix cargo-buildlib-,kfs $(KERNEL_MODULE_NAMES))

.PHONY : $(CARGO_TARGETS)
$(CARGO_TARGETS) :
	@echo CARGO lib$(subst cargo-buildlib-,,$@).a
	@cargo rustc -p $(subst cargo-buildlib-,,$@) $(CARGO_FLAG) -- $(RUSTC_FLAG)