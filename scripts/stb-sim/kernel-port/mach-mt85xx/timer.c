// SPDX-License-Identifier: GPL-2.0-or-later
/*
 * MT85xx timer driver - PLACEHOLDER.
 *
 * The MT85xx (MT8653) uses a custom timer block that is not documented
 * publicly. This file is a stub to keep the build coherent and to document
 * what a real implementation would need.
 *
 * A real driver would:
 *   1. ioremap the timer registers (base from DT)
 *   2. register a clocksource (read count) + clockevent (program compare)
 *   3. request and handle the timer IRQ
 *
 * Reference: the vendor 2.6.35 kernel has this in arch/arm/mach-mt85xx/timer.c
 */

#include <linux/init.h>
#include <linux/clocksource.h>
#include <linux/clockchips.h>
#include <linux/interrupt.h>
#include <linux/io.h>
#include <linux/of.h>
#include <linux/of_address.h>
#include <linux/of_irq.h>

static int __init mt85xx_timer_probe(struct device_node *np)
{
	/*
	 * TODO: implement. Without hardware documentation this cannot be
	 * completed. Return -ENODEV so the kernel falls back gracefully.
	 */
	pr_info("mt85xx: timer driver is a stub (no hardware support)\n");
	return -ENODEV;
}

TIMER_OF_DECLARE(mt85xx_timer, "mediatek,mt85xx-timer", mt85xx_timer_probe);
