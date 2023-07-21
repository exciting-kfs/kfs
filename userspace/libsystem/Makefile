NAME = $(BUILDDIR)/libsystem.a

C_SRCS := crt0.c
ASM_SRCS := syscall.S

MODULE_NAME := $(shell basename $(CURDIR)) 
OBJS_PREFIX := $(join $(BUILDDIR)/$(MODULE_NAME),_)

OBJS := $(C_SRCS:.c=.o) $(ASM_SRCS:.S=.o)
OBJS := $(addprefix $(OBJS_PREFIX),$(OBJS))

.PHONY : all
all : $(NAME)

$(NAME) : $(OBJS)
	@echo ' 'AR $(notdir $@)
	@$(AR) -rcs $@ $^

$(OBJS_PREFIX)%.o : %.c
	@echo ' 'CC $(notdir $^)
	@$(CC) $(CFLAGS) -c -o $@ $^

$(OBJS_PREFIX)%.o : %.S
	@echo ' 'AS $(notdir $^)
	@$(AS) -felf -o $@ $^
