/* ************************************************************************** */
/*                                                                            */
/*                                                        :::      ::::::::   */
/*   opts_reader.c                                      :+:      :+:    :+:   */
/*                                                    +:+ +:+         +:+     */
/*   By: mypark <mypark@student.42seoul.kr>         +#+  +:+       +#+        */
/*                                                +#+#+#+#+#+   +#+           */
/*   Created: 2022/01/05 20:04:33 by mypark            #+#    #+#             */
/*   Updated: 2022/01/05 20:04:35 by mypark           ###   ########.fr       */
/*                                                                            */
/* ************************************************************************** */

#include "kfs/libft.h"

t_state	opts_reader(int *sum, char c, t_optable *otb, va_list ap)
{
	t_state	s;

	if (is_in(c, "0123456789#-. +"))
		s = opts_parser(c, otb);
	else
		s = param_printer(sum, c, otb, ap);
	return (s);
}
