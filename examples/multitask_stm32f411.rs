#![no_std]
#![no_main]
extern crate alloc;

//use panic_halt as _;

use stm32f4xx_hal::prelude::*;
use xtask::arch::cortex_m::rt;
use xtask::bsp::greenpill;
use xtask::bsp::greenpill::hal::prelude::*;
use xtask::bsp::greenpill::led::Led;
use xtask::bsp::greenpill::stdout;
use xtask::chip::{CPU_CLOCK_HZ, SYSTICK_CLOCK_HZ};
use xtask::prelude::*;

#[inline]
pub fn stack_start() -> *mut u32 {
    extern "C" {
        static mut _stack_start: u32;
    }
    unsafe { &mut _stack_start }
}

fn init() {
    xtask::init_logger();
    let start_addr = rt::heap_start() as usize;
    let stack_addr = stack_start() as usize;
    xtask::init_heap(start_addr, stack_addr - 4 * 1024 - start_addr);

    if let Some((_cp, dp)) = greenpill::take() {
        let rcc = dp.RCC.constrain();
        let clocks = rcc
            .cfgr
            .sysclk((CPU_CLOCK_HZ as u32).Hz())
            .hclk((SYSTICK_CLOCK_HZ as u32).Hz())
            .freeze();

        let gpioa = dp.GPIOA.split();
        let gpioc = dp.GPIOC.split();
        let led = Led::new(gpioc.pc13);
        let tx = dp
            .USART1
            .tx(gpioa.pa9.into_alternate(), 115200.bps(), &clocks)
            .unwrap();
        stdout::use_tx1(tx);
        log::info!("clocks {}", clocks.sysclk());
        log::info!("STM32F411CEU6 initialize ok");

        example_led(led);
    }
}

#[rt::entry]
fn main() -> ! {
    init();

    //启动多任务
    example_task();
    //启动调度器
    xtask::start()
}

fn example_task() {
    xtask::spawn(|| {
        for i in 0..10 {
            log::info!("{} 循环测试任务0", i + 1);
            xtask::sleep_ms(1000);
        }
    });
    xtask::spawn(|| {
        for i in 0..50 {
            log::info!("{} 循环测试任务1", i + 1);
            xtask::sleep_ms(1000);
        }
    });

    xtask::spawn(|| {
        for i in 0..100 {
            log::info!("{} 循环测试任务2", i + 1);
            xtask::sleep_ms(1000);
        }
    });

    xtask::spawn(|| {
        for i in 0..500 {
            log::info!("{} 循环测试任务4", i + 1);
            xtask::sleep_ms(1000);
        }
    });

    xtask::spawn(|| loop {
        log::info!("死循环测试任务 {}", tick());
        xtask::sleep_ms(1000);
    });
}

fn example_led(mut blue: Led) {
    TaskBuilder::new()
        .name("blue")
        .priority(1)
        .spawn(move || loop {
            blue.on();
            xtask::sleep_ms(500);
            blue.off();
            xtask::sleep_ms(500);
        });
}
