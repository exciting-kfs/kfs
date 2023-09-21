/* ************************************************************************** */
/*                                                                            */
/*                                                        :::      ::::::::   */
/*   libft.h                                            :+:      :+:    :+:   */
/*                                                    +:+ +:+         +:+     */
/*   By: mypark <mypark@student.42.fr>              +#+  +:+       +#+        */
/*                                                +#+#+#+#+#+   +#+           */
/*   Created: 2021/11/25 18:45:54 by mypark            #+#    #+#             */
/*   Updated: 2022/04/06 19:59:48 by mypark           ###   ########.fr       */
/*                                                                            */
/* ************************************************************************** */

#ifndef LIBFT_H
#define LIBFT_H
#include <stdarg.h>
#include <unistd.h>

typedef struct s_list {
	void *content;
	struct s_list *next;
} t_list;

int ft_atoi(const char *str);
int ft_isalpha(int c);
int ft_isalnum(int c);
int ft_isdigit(int c);
int ft_isascii(int c);
int ft_isprint(int c);
int ft_toupper(int c);
int ft_tolower(int c);
int ft_strncmp(const char *s1, const char *s2, size_t n);
int ft_memcmp(const void *s1, const void *s2, size_t n);
void ft_striteri(char *s, void (*f)(unsigned int, char *));
void ft_bzero(void *s, size_t n);
size_t ft_strlen(const char *s);
size_t ft_strlcpy(char *dst, const char *src, size_t dstsize);
size_t ft_strlcat(char *dst, const char *src, size_t dstsize);
char *ft_strchr(const char *s, int c);
int ft_strchri(const char *s, int c);
char *ft_strrchr(const char *s, int c);
void *ft_memset(void *s, int c, size_t n);
void *ft_memchr(const void *s, int c, size_t n);
void *ft_memcpy(void *s1, const void *s2, size_t n);
void *ft_memmove(void *s1, const void *s2, size_t n);
void ft_putchar_fd(char c, int fd);
void ft_putendl_fd(char *s, int fd);
void ft_putstr_fd(char *s, int fd);
void ft_putnbr_fd(int n, int fd);

void ft_strtoupper(char *s);
int ft_wordcount(char **words);

typedef enum e_state { c_write, o_read } t_state;

typedef enum e_parser_state { flags, width, preci } t_parser_state;

enum e_align { left, right };

typedef struct s_stack {
	int sp;
	int bp;
	char pool[256];
} t_stack;

typedef struct s_optable {
	int space;
	int plus;
	int base;
	int zero;
	enum e_align align;
	size_t width;
	size_t preci;
	int p_chk;
	int init;
	void (*action[3][128])(char, t_parser_state *, struct s_optable *);
} t_optable;

int ft_printf(const char *fmt, ...);
int is_in(char c, char *s);
int my_putnbr(char type, const t_optable *otb, va_list ap);
int print_ws(int len, char w);
int print_var(char type, const t_optable *otb, va_list ap);
int print_stack(t_stack *nbrst, const t_optable *otb);
t_state param_printer(int *sum, char type, t_optable *otb, va_list ap);
t_state opts_parser(char c, t_optable *otb);
t_state opts_reader(int *sum, char c, t_optable *otb, va_list ap);
t_state char_writer(int *sum, const char c);
void opts_correction(int type, t_optable *otb);
void opts_initialization(t_optable *otb);
void ntoa_base(int type, ssize_t x, int base, t_stack *nbrst);
void fill_action(t_optable *otb);
void append_width(char c, t_parser_state *ps, t_optable *otb);
void append_preci(char c, t_parser_state *ps, t_optable *otb);
void goto_width(char c, t_parser_state *ps, t_optable *otb);
void goto_preci(char c, t_parser_state *ps, t_optable *otb);
void fill_optable(char c, t_parser_state *ps, t_optable *otb);

#endif
