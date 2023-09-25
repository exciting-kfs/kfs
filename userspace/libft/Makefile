NAME = $(BUILDDIR)/libft.a

C_SRCS := actions.c \
	char_writer.c \
	ft_atoi.c \
	ft_bzero.c \
	ft_isalnum.c \
	ft_isalpha.c \
	ft_isascii.c \
	ft_isdigit.c \
	ft_isprint.c \
	ft_memchr.c \
	ft_memcmp.c \
	ft_memcpy.c \
	ft_memmove.c \
	ft_memset.c \
	ft_printf.c \
	ft_putchar_fd.c \
	ft_putendl_fd.c \
	ft_putnbr_fd.c \
	ft_putstr_fd.c \
	ft_strchr.c \
	ft_strchri.c \
	ft_striteri.c \
	ft_strlcat.c \
	ft_strlcpy.c \
	ft_strlen.c \
	ft_strncmp.c \
	ft_strrchr.c \
	ft_strtoupper.c \
	ft_tolower.c \
	ft_toupper.c \
	ft_wordcount.c \
	is_in.c \
	my_putnbr.c \
	ntoa.c \
	opts_handle.c \
	opts_parser.c \
	opts_reader.c \
	param_printer.c \
	print_stack.c \
	print_var.c \
	print_ws.c \
	utils.c



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
