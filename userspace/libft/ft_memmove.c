/* ************************************************************************** */
/*                                                                            */
/*                                                        :::      ::::::::   */
/*   ft_memmove.c                                       :+:      :+:    :+:   */
/*                                                    +:+ +:+         +:+     */
/*   By: mypark <mypark@student.42seoul.kr>         +#+  +:+       +#+        */
/*                                                +#+#+#+#+#+   +#+           */
/*   Created: 2021/11/17 14:30:50 by mypark            #+#    #+#             */
/*   Updated: 2021/12/13 20:03:08 by mypark           ###   ########.fr       */
/*                                                                            */
/* ************************************************************************** */

#include "kfs/libft.h"

static void	directional_copy(unsigned char *dst, unsigned char *src, \
size_t n, int add)
{
	int	s;
	int	e;

	if (add == 1)
	{
		s = 0;
		e = (int) n;
	}
	else
	{
		s = (int) n - 1;
		e = -1;
	}
	while (s != e)
	{
		dst[s] = src[s];
		s += add;
	}
}

void	*ft_memmove(void *s1, const void *s2, size_t n)
{
	unsigned char	*dst;
	unsigned char	*src;

	if (s1 == NULL && s2 == NULL)
		return (NULL);
	dst = (unsigned char *)s1;
	src = (unsigned char *)s2;
	if (src + n > dst && src <= dst)
		directional_copy(dst, src, n, -1);
	else
		directional_copy(dst, src, n, 1);
	return (s1);
}
