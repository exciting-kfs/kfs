/* ************************************************************************** */
/*                                                                            */
/*                                                        :::      ::::::::   */
/*   ft_atoi.c                                          :+:      :+:    :+:   */
/*                                                    +:+ +:+         +:+     */
/*   By: mypark <mypark@student.42seoul.kr>         +#+  +:+       +#+        */
/*                                                +#+#+#+#+#+   +#+           */
/*   Created: 2021/11/28 15:39:33 by mypark            #+#    #+#             */
/*   Updated: 2021/12/20 16:41:44 by mypark           ###   ########.fr       */
/*                                                                            */
/* ************************************************************************** */

#include "kfs/libft.h"

static int is_space(char c) {
	if (c == ' ' || c == '\n' || c == '\t' || c == '\r' || c == '\v' || c == '\f')
		return (1);
	return (0);
}

static int get_sign(const char **str) {
	int sign;

	sign = 1;
	if (**str == '+' || **str == '-') {
		if (**str == '-')
			sign = -1;
		(*str)++;
	}
	return (sign);
}

static void jump_spaces(const char **str) {
	while (is_space(**str))
		(*str)++;
}

int ft_atoi(const char *str) {
	int sign;
	long int num;
	long int pre;

	jump_spaces(&str);
	sign = get_sign(&str);
	num = 0;
	while (ft_isdigit(*str)) {
		pre = num;
		num = num * 10 + (*str - '0');
		if (num < pre) {
			if (sign == 1)
				return (-1);
			else
				return (0);
		}
		str++;
	}
	return (sign * num);
}
