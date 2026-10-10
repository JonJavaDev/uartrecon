// SPDX-License-Identifier: GPL-2.0-or-later
/*
 * Device Tree support for MediaTek MT85xx (STB) SoCs.
 *
 * SoC:     MediaTek MT8653 (MT85xx family)
 * CPU:     ARM1176JZF-S (ARMv6K), part 0xb76
 * Device:  ZTE B700V5S1 set-top box (also found in similar MT85xx STBs)
 *
 * ------------------------------------------------------------------------
 * STATUS: EXPERIMENTAL SKELETON - NOT BOOTABLE ON REAL HARDWARE
 * ------------------------------------------------------------------------
 * Mainline Linux does not support MT85xx. This file provides the platform
 * descriptor so that a DT-based bring-up can be attempted, but the
 * SoC-specific drivers do not exist upstream:
 *
 *   [ ] Interrupt controller   (vendor: custom VIC-like block)
 *   [ ] Timer / clocksource    (vendor: custom MTK timer)
 *   [ ] Clock controller       (vendor: custom)
 *   [ ] GPIO / pinctrl         (vendor: custom)
 *   [ ] NAND flash controller  (vendor: mtk_nand, proprietary)
 *   [ ] Ethernet MAC "star"    (vendor: star.ko, proprietary)
 *   [ ] USB host "MtkUsbHcd"   (vendor: proprietary)
 *   [ ] Video / HDMI           (vendor: proprietary)
 *   [ ] Watchdog               (vendor: /proc/net/monitor, proprietary)
 *
 * To make this real you would need, at minimum:
 *   1. The MT8653 datasheet / register manual (NOT public).
 *   2. The vendor 2.6.35 kernel source for MT85xx (to port drivers from).
 *   3. Significant time to write and test each driver.
 *
 * This skeleton is published as a starting point / documentation of the
 * effort required, not as a working kernel.
 */

#include <linux/init.h>
#include <linux/io.h>
#include <linux/of.h>
#include <linux/of_clk.h>
#include <linux/clocksource.h>
#include <asm/mach/arch.h>

/*
 * NOTE: The MT85xx timer is a custom block. The address below is a
 * PLACEHOLDER taken from the vendor kernel and is NOT verified.
 */
#define MT85XX_TIMER_BASE	0x00100000

static void __init mt85xx_init_time(void)
{
	/*
	 * TODO: register the MT85xx timer as clocksource + clockevent.
	 *
	 * Without a real driver this will not provide a usable tick, so a
	 * real port must implement arch/arm/mach-mt85xx/timer.c.
	 */
	of_clk_init(NULL);
	timer_probe();
}

static void __init mt85xx_init_early(void)
{
	/*
	 * TODO: early debug UART mapping (8250-compatible on vendor kernel).
	 */
}

static const char * const mt85xx_dt_compat[] = {
	"mediatek,mt8653",
	"mediatek,mt85xx",
	NULL,
};

DT_MACHINE_START(MT85XX_DT, "MediaTek MT85xx (Device Tree)")
	.dt_compat	= mt85xx_dt_compat,
	.init_early	= mt85xx_init_early,
	.init_time	= mt85xx_init_time,
MACHINE_END
