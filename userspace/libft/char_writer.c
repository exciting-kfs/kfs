/* ************************************************************************** */
/*                                                                            */
/*                                                        :::      ::::::::   */
/*   char_writer.c                                      :+:      :+:    :+:   */
/*                                                    +:+ +:+         +:+     */
/*   By: mypark <mypark@student.42seoul.kr>         +#+  +:+       +#+        */
/*                                                +#+#+#+#+#+   +#+           */
/*   Created: 2021/12/21 04:59:49 by mypark            #+#    #+#             */
/*   Updated: 2022/01/05 18:52:24 by mypark           ###   ########.fr       */
/*                                                                            */
/* ************************************************************************** */

#include "kfs/libft.h"

t_state	char_writer(int *sum, const char c)
{
	if (c == '%')
		return (o_read);
	(*sum)++;
	write(1, &c, 1);
	return (c_write);
}
