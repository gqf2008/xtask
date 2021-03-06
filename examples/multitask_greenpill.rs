#![no_std]
#![no_main]
extern crate alloc;

//use panic_halt as _;

use xtask::arch::cortex_m::rt;
use xtask::bsp::greenpill;
use xtask::bsp::greenpill::hal::prelude::*;
use xtask::bsp::greenpill::led::Led;
use xtask::bsp::greenpill::stdout;
use xtask::prelude::*;

fn init() {
    let start_addr = rt::heap_start() as usize;
    //4k留给主栈
    xtask::init(start_addr, 256 * 1024);

    if let Some((_cp, dp)) = greenpill::take() {
        let rcc = dp.RCC.constrain();
        let clocks = rcc.cfgr.freeze();

        let gpioa = dp.GPIOA.split();
        let gpioc = dp.GPIOC.split();
        let led = Led::new(gpioc.pc13);
        let tx = dp
            .USART1
            .tx(gpioa.pa9.into_alternate(), 115200.bps(), &clocks)
            .unwrap();
        stdout::use_tx1(tx);
        log::info!("clocks {}", clocks.sysclk());
        log::info!("Greenpill initialize ok");
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
