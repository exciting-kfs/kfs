MODULE_NAME := $(shell basename $(CURDIR)) 

OBJS_PREFIX := $(join $(BUILDDIR)/$(MODULE_NAME),_)
OBJS := $(addprefix $(OBJS_PREFIX),$(SRCS:.c=.o))

$(BUILDDIR)/$(NAME) : $(OBJS)
	@echo ' 'LD $(NAME)
	@$(LD) $(LDFLAGS) -L$(BUILDDIR) -lsystem -o $@ $^ -lft

$(OBJS_PREFIX)%.o : %.c
	@echo ' 'CC $(notdir $^)
	@$(CC) $(CFLAGS) -c $^ -o $@
