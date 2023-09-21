/* ************************************************************************** */
/*                                                                            */
/*                                                        :::      ::::::::   */
/*   ft_putnbr_fd.c                                     :+:      :+:    :+:   */
/*                                                    +:+ +:+         +:+     */
/*   By: mypark <mypark@student.42seoul.kr>         +#+  +:+       +#+        */
/*                                                +#+#+#+#+#+   +#+           */
/*   Created: 2021/12/01 18:29:37 by mypark            #+#    #+#             */
/*   Updated: 2021/12/02 20:57:12 by mypark           ###   ########.fr       */
/*                                                                            */
/* ************************************************************************** */

#include "kfs/libft.h"

static int	ft_abs(int x)
{
	if (x < 0)
		return (x * -1);
	return (x);
}

void	recursive_putnbr(int n, int fd)
{
	char	c;

	if (n / 10 > 0)
	{
		recursive_putnbr(n / 10, fd);
	}
	c = n % 10 + '0';
	write(fd, &c, 1);
}

void	ft_putnbr_fd(int n, int fd)
{
	if (n == -2147483648)
	{
		write(fd, "-2147483648", 11);
		return ;
	}
	if (n == 0)
	{
		write(fd, "0", 1);
		return ;
	}
	if (n < 0)
		write(fd, "-", 1);
	recursive_putnbr(ft_abs(n), fd);
}
