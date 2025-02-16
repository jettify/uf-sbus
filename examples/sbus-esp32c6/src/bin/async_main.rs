#![no_std]
#![no_main]

use defmt::info;
use embassy_executor::Spawner;
use embassy_time::{with_timeout, Duration, TimeoutError, Timer};
use esp_hal::clock::CpuClock;
use esp_hal::gpio::{Level, Output};
use esp_hal::uart::{Config, DataBits, Parity, StopBits, Uart};
use sbus_protocol::SbusParser;
use {defmt_rtt as _, esp_backtrace as _};

#[esp_hal_embassy::main]
async fn main(_spawner: Spawner) {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);
    let timer0 = esp_hal::timer::systimer::SystemTimer::new(peripherals.SYSTIMER);
    esp_hal_embassy::init(timer0.alarm0);
    info!("Embassy initialized!");

    let uart_conf = Config::default()
        .with_baudrate(100_000)
        .with_data_bits(DataBits::_8)
        .with_parity(Parity::Even)
        .with_stop_bits(StopBits::_2);

    let (rx_pin, _) = peripherals.GPIO20.split();
    let (_, tx_pin) = peripherals.GPIO19.split();

    let uart0 = Uart::new(peripherals.UART0, uart_conf)
        .unwrap()
        .with_rx(rx_pin.inverted())
        .with_tx(tx_pin.inverted());

    let mut uart = uart0.into_async();

    let mut buf: [u8; 25] = [0; 25];
    let mut sbus = SbusParser::new();

    info!("Starting reading loop!");
    let mut led = Output::new(peripherals.GPIO8, Level::Low);
    led.set_high();
    loop {
        let result = with_timeout(Duration::from_millis(500), uart.read_async(&mut buf)).await;
        match result {
            Ok(Ok(size)) => {
                for result in sbus.iter_packets(&buf[..size]) {
                    match result {
                        Ok(packet) => info!("{:?}", packet.channels),
                        Err(e) => info!("{:?}", e),
                    }
                }
            }
            Ok(Err(read_error)) => {
                info!("reading error {:?}", read_error)
            }
            Err(TimeoutError) => sbus.reset(),
        }
    }
}
