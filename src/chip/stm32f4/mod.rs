mod port;
pub mod stdout;
use super::{CPU_CLOCK_HZ, SYSTICK_CLOCK_HZ, TICK_CLOCK_HZ};

use crate::port::Portable;
use crate::task::Task;
use crate::{isr_sprintln, sprintln, CriticalSection};
use core::arch::asm;
use cortex_m::peripheral::scb::SystemHandler;
use cortex_m::peripheral::syst::SystClkSource;
use cortex_m::peripheral::{SCB, SYST};

pub struct STM32F4Porting;

unsafe fn setup_intrrupt() {
    sprintln!("setup_intrrupt");
    cortex_m::peripheral::SCB::priority(SystemHandler::PendSV, 0xff);
    cortex_m::peripheral::SCB::priority(SystemHandler::SysTick, 0xff);
    cortex_m::peripheral::SYST::clock_source(SystClkSource::External);
    cortex_m::peripheral::SYST::reload(SYSTICK_CLOCK_HZ as u32 / TICK_CLOCK_HZ as u32 - 1);
    cortex_m::peripheral::SYST::reset_current();
    cortex_m::peripheral::SYST::open_counter();
    cortex_m::peripheral::SYST::open_interrupt();
}

trait ScbExt {
    unsafe fn priority(system_handler: SystemHandler, prio: u8);
}

impl ScbExt for SCB {
    #[inline]
    unsafe fn priority(system_handler: SystemHandler, prio: u8) {
        let index = system_handler as u8;

        #[cfg(not(armv6m))]
        {
            // NOTE(unsafe): Index is bounded to [4,15] by SystemHandler design.
            // TODO: Review it after rust-lang/rust/issues/13926 will be fixed.
            let priority_ref = (*Self::ptr()).shpr.get_unchecked(usize::from(index - 4));

            priority_ref.write(prio)
        }

        #[cfg(armv6m)]
        {
            // NOTE(unsafe): Index is bounded to [11,15] by SystemHandler design.
            // TODO: Review it after rust-lang/rust/issues/13926 will be fixed.
            let priority_ref = (*Self::ptr())
                .shpr
                .get_unchecked(usize::from((index - 8) / 4));

            priority_ref.modify(|value| {
                let shift = 8 * (index % 4);
                let mask = 0x0000_00ff << shift;
                let prio = u32::from(prio) << shift;

                (value & !mask) | prio
            });
        }
    }
}

trait SystExt {
    fn open_interrupt();
    fn open_counter();
    fn reset_current();
    fn current() -> u32;
    fn reload(val: u32);
    fn clock_source(src: SystClkSource);
}

impl SystExt for SYST {
    #[inline]
    fn clock_source(src: SystClkSource) {
        match src {
            SystClkSource::External => unsafe { (*Self::ptr()).csr.modify(|v| v & !(1 << 2)) },
            SystClkSource::Core => unsafe { (*Self::ptr()).csr.modify(|v| v | (1 << 2)) },
        }
    }
    #[inline]
    fn open_interrupt() {
        unsafe { (*Self::ptr()).csr.modify(|v| v | (1 << 1)) }
    }
    #[inline]
    fn open_counter() {
        unsafe { (*Self::ptr()).csr.modify(|v| v | (1 << 0)) }
    }
    #[inline]
    fn reset_current() {
        unsafe { (*Self::ptr()).cvr.write(0) }
    }
    #[inline]
    fn current() -> u32 {
        unsafe { (*Self::ptr()).cvr.read() }
    }
    #[inline]
    fn reload(value: u32) {
        unsafe { (*Self::ptr()).rvr.write(value) }
    }
}

impl Portable for STM32F4Porting {
    /// 完全内存屏障
    /// 保证在屏障之前的任何存储操作先于屏障之后的代码执行。
    fn barrier() {
        unimplemented!()
    }
    fn free<F, R>(f: F) -> R
    where
        F: FnOnce(&CriticalSection) -> R,
    {
        unsafe { cortex_m::interrupt::free(|_| f(&CriticalSection::new())) }
    }

    /// 开全局中断
    fn enable_interrupt() {
        unsafe { cortex_m::interrupt::enable() }
        // unsafe {
        //     asm!(
        //         "
        //     cpsie i # 开中断
        //     cpsie f # 开异常
        //     dsb
        //     isb
        //     "
        //     );
        // }
    }
    /// 关全局中断
    fn disable_interrupt() {
        crate::arch::cortex_m::interrupt::disable()
        // unsafe {
        //     asm!(
        //         "
        // cpsid i # 关中断
        // cpsid f # 关异常
        // dsb
        // isb
        // "
        //     );
        // }
    }
    /// 启动调度器
    fn start_scheduler() -> ! {
        //配置中断，这个函数就是定时中断和软中断使能
        sprintln!("start_scheduler");
        //从任务栈恢复CPU状态，汇编实现
        unsafe {
            setup_intrrupt();
            asm!(include_str!("restore_ctx.S"))
        };
        panic!("~!@#$%^&*()_")
    }
    /// 软中断
    fn irq() {
        cortex_m::peripheral::SCB::set_pendsv();
        // let ptr = NVIC_INT_CTL_ADDR as *mut u32;
        // unsafe {
        //     ptr.write_volatile(1 << 28);
        //     crate::arch::cortex_m::asm::dsb();
        //     crate::arch::cortex_m::asm::isb();
        // }
    }

    fn disable_irq() {
        cortex_m::peripheral::SCB::clear_pendsv();
        // let ptr = NVIC_INT_CTL_ADDR as *mut u32;
        // unsafe {
        //     ptr.write_volatile(1 << 27);
        //     crate::arch::cortex_m::asm::dsb();
        //     crate::arch::cortex_m::asm::isb();
        // }
    }

    /// 获取rtc tick
    fn systick() -> u64 {
        unsafe { core::ptr::read_volatile(&port::SYSTICKS) }
    }
    /// 硬件延时，单位us
    fn delay_us(us: u64) {
        let clock = (us * (CPU_CLOCK_HZ as u64)) / 1_000_000;
        cortex_m::asm::delay(clock as u32);
    }
    /// 保存任务环境到任务栈
    fn save_context(task: &mut Task) {
        unsafe {
            //任务栈指针移到栈顶，也就是数组的最后一个元素起始位置
            let mut sp = task.stack.add(task.stack_size - 1);
            sp = sp.offset(-1);
            sp.write_volatile(0x01000000); /* xPSR */
            sp = sp.offset(-1);
            sp.write_volatile((task.entry as *const ()).addr()); /* PC */
            sp = sp.offset(-1);
            sp.write_volatile((port::task_exit as *const ()).addr()); /* LR */
            sp = sp.offset(-5); /* R12, R3, R2 and R1. */
            sp.write_volatile(task.args.addr()); /* R0 */
            sp = sp.offset(-1);
            sp.write_volatile(0xfffffffd);
            sp = sp.offset(-8); /* R11, R10, R9, R8, R7, R6, R5 and R4. */
            task.sp = sp.addr();
            sprintln!("top of stack {:#08x}", task.sp);
        }
    }
    /// 打印文本函数
    fn printf(str: &str) {
        stdout::write_str(str)
    }
}
