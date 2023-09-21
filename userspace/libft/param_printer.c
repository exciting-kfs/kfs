/* ************************************************************************** */
/*                                                                            */
/*                                                        :::      ::::::::   */
/*   param_printer.c                                   :+:      :+:    :+:   */
/*                                                    +:+ +:+         +:+     */
/*   By: mypark <mypark@student.42seoul.kr>         +#+  +:+       +#+        */
/*                                                +#+#+#+#+#+   +#+           */
/*   Created: 2021/12/21 04:59:49 by mypark            #+#    #+#             */
/*   Updated: 2021/12/28 20:55:03 by mypark           ###   ########.fr       */
/*                                                                            */
/* ************************************************************************** */

#include "kfs/libft.h"

t_state	param_printer(int *sum, char type, t_optable *otb, va_list ap)
{
	int	len;

	opts_correction(type, otb);
	len = print_var(type, otb, ap);
	(*sum) += len;
	opts_initialization(otb);
	return (c_write);
}
