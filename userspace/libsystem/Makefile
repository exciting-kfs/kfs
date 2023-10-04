NAME = $(BUILDDIR)/libsystem.a

C_SRCS := crt0.c

MODULE_NAME := $(shell basename $(CURDIR)) 
OBJS_PREFIX := $(join $(BUILDDIR)/$(MODULE_NAME),_)

OBJS := $(C_SRCS:.c=.o)
OBJS := $(addprefix $(OBJS_PREFIX),$(OBJS))

.PHONY : all
all : $(NAME)

$(NAME) : $(OBJS)
	@echo ' 'AR $(notdir $@)
	@$(AR) -rcs $@ $^

$(OBJS_PREFIX)%.o : %.c
	@echo ' 'CC $(notdir $^)
	@$(CC) $(CFLAGS) -c -o $@ $^
