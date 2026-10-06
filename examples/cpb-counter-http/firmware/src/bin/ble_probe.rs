//! spike(no_std): BLE go/no-go for the Circuit Playground Bluefruit.
//!
//! Does Nordic's SoftDevice Controller (nrf-sdc + MPSL, linked into this image) work
//! when the board boots through the MBR and a resident but disabled S140 6.1.1, which
//! forward every interrupt to us? cpb-counter showed that for GPIOTE; MPSL's RADIO,
//! TIMER0 and RTC0 handlers are timing-critical.
//!
//! It advertises as "CPB Counter" with the gateway service (`tx` notify, `rx` write
//! without response) and echoes every `rx` write back as a `tx` notification.
//!
//! Pixel 0: dim blue while advertising, green while connected. Pixel 1 flashes white for
//! each echoed write. All red: the stack returned an error (the main loop stops).

#![no_std]
#![no_main]

#[path = "../neopixel.rs"]
mod neopixel;

use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_nrf::gpio::{Level, Output, OutputDrive};
use embassy_nrf::mode::Async;
use embassy_nrf::peripherals::RNG;
use embassy_nrf::{bind_interrupts, rng};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::Timer;
use heapless::Vec;
use nrf_sdc::mpsl::MultiprotocolServiceLayer;
use nrf_sdc::{self as sdc, mpsl};
use panic_halt as _;

// The crate's other binaries use `alloc`, so one must exist; this binary never allocates.
#[global_allocator]
static HEAP: embedded_alloc::LlffHeap = embedded_alloc::LlffHeap::empty();
use static_cell::StaticCell;
use trouble_host::prelude::*;

use neopixel::{Frame, NeoPixels, PIXELS};

bind_interrupts!(struct Irqs {
    RNG => rng::InterruptHandler<RNG>;
    EGU0_SWI0 => mpsl::LowPrioInterruptHandler;
    CLOCK_POWER => mpsl::ClockInterruptHandler;
    RADIO => mpsl::HighPrioInterruptHandler;
    TIMER0 => mpsl::HighPrioInterruptHandler;
    RTC0 => mpsl::HighPrioInterruptHandler;
});

const SERVICE_UUID: u128 = 0x744aa621_b4b9_4751_91c0_a43b54bb1544;
/// The largest value one ATT write or notification can carry with the default MTU pool.
const VALUE_MAX: usize = 244;

#[gatt_server]
struct Server {
    gateway: GatewayService,
}

/// Device <-> gateway link: framed messages, chunked to the ATT payload size.
#[gatt_service(uuid = "744aa621-b4b9-4751-91c0-a43b54bb1544")]
struct GatewayService {
    /// Device -> gateway.
    #[characteristic(uuid = "ad229cac-4e64-4208-b76b-ed536a1e47c5", notify)]
    tx: Vec<u8, VALUE_MAX>,
    /// Gateway -> device.
    #[characteristic(uuid = "cb4206ec-fe9c-4fea-ae65-2e7c5b56515b", write_without_response, write)]
    rx: Vec<u8, VALUE_MAX>,
}

#[derive(Clone, Copy)]
enum Status {
    Advertising,
    Connected,
    Echo,
    Error,
}

static STATUS: Signal<CriticalSectionRawMutex, Status> = Signal::new();

#[embassy_executor::task]
async fn mpsl_task(mpsl: &'static MultiprotocolServiceLayer<'static>) -> ! {
    mpsl.run().await
}

#[embassy_executor::task]
async fn pixels_task(mut pixels: NeoPixels<'static>) -> ! {
    let mut base = [0, 0, 8];
    loop {
        let mut frame: Frame = [[0; 3]; PIXELS];
        match STATUS.wait().await {
            Status::Advertising => base = [0, 0, 8],
            Status::Connected => base = [0, 12, 0],
            Status::Echo => {
                frame[0] = base;
                frame[1] = [16, 16, 16];
                pixels.show(&frame);
                Timer::after_millis(80).await;
                frame[1] = [0; 3];
            }
            Status::Error => {
                pixels.show(&[[24, 0, 0]; PIXELS]);
                continue;
            }
        }
        frame[0] = base;
        pixels.show(&frame);
    }
}

fn build_sdc<'d, const N: usize>(
    p: nrf_sdc::Peripherals<'d>,
    rng: &'d mut rng::Rng<Async>,
    mpsl: &'d MultiprotocolServiceLayer,
    mem: &'d mut sdc::Mem<N>,
) -> Result<nrf_sdc::SoftdeviceController<'d>, nrf_sdc::Error> {
    sdc::Builder::new()?
        .support_adv()
        .support_peripheral()
        .peripheral_count(1)?
        .buffer_cfg(
            DefaultPacketPool::MTU as u16,
            DefaultPacketPool::MTU as u16,
            3,
            3,
        )?
        .build(p, rng, mpsl, mem)
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_nrf::init(Default::default());

    // Power the NeoPixels, and show "starting" (dim white) before touching the radio, so
    // a hang in MPSL/SDC set-up is visible.
    let _power = Output::new(p.P0_06, Level::Low, OutputDrive::Standard);
    let mut pixels = NeoPixels::new(p.PWM0, p.P0_13);
    let mut frame: Frame = [[0; 3]; PIXELS];
    frame[0] = [6, 6, 6];
    pixels.show(&frame);
    spawner.spawn(pixels_task(pixels).unwrap());

    let mpsl_p =
        mpsl::Peripherals::new(p.RTC0, p.TIMER0, p.TEMP, p.PPI_CH19, p.PPI_CH30, p.PPI_CH31);
    // The CPB has no 32 kHz crystal (CircuitPython: BOARD_HAS_32KHZ_XTAL 0), so LFCLK
    // runs from the RC oscillator with periodic calibration.
    let lfclk_cfg = mpsl::raw::mpsl_clock_lfclk_cfg_t {
        source: mpsl::raw::MPSL_CLOCK_LF_SRC_RC as u8,
        rc_ctiv: mpsl::raw::MPSL_RECOMMENDED_RC_CTIV as u8,
        rc_temp_ctiv: mpsl::raw::MPSL_RECOMMENDED_RC_TEMP_CTIV as u8,
        accuracy_ppm: mpsl::raw::MPSL_DEFAULT_CLOCK_ACCURACY_PPM as u16,
        skip_wait_lfclk_started: mpsl::raw::MPSL_DEFAULT_SKIP_WAIT_LFCLK_STARTED != 0,
    };
    static MPSL: StaticCell<MultiprotocolServiceLayer> = StaticCell::new();
    let Ok(mpsl) = MultiprotocolServiceLayer::new(mpsl_p, Irqs, lfclk_cfg) else {
        STATUS.signal(Status::Error);
        return;
    };
    let mpsl = MPSL.init(mpsl);
    spawner.spawn(mpsl_task(mpsl).unwrap());

    let sdc_p = sdc::Peripherals::new(
        p.PPI_CH17, p.PPI_CH18, p.PPI_CH20, p.PPI_CH21, p.PPI_CH22, p.PPI_CH23, p.PPI_CH24,
        p.PPI_CH25, p.PPI_CH26, p.PPI_CH27, p.PPI_CH28, p.PPI_CH29,
    );
    let mut rng = rng::Rng::new(p.RNG, Irqs);
    let mut sdc_mem = sdc::Mem::<4720>::new();
    let Ok(sdc) = build_sdc(sdc_p, &mut rng, mpsl, &mut sdc_mem) else {
        STATUS.signal(Status::Error);
        return;
    };

    run(sdc).await;
}

async fn run<C: Controller>(controller: C) {
    // A fixed static random address (top two bits set), so Chrome sees the same device
    // across resets.
    let address = Address::random([0x5c, 0x0c, 0x4e, 0x75, 0x0b, 0xc3]);
    let mut resources: HostResources<DefaultPacketPool, 1, 2> = HostResources::new();
    let stack = trouble_host::new(controller, &mut resources)
        .set_random_address(address)
        .build();
    let mut runner = stack.runner();
    let mut peripheral = stack.peripheral();

    let Ok(server) = Server::new_with_config(GapConfig::Peripheral(PeripheralConfig {
        name: "CPB Counter",
        appearance: &appearance::UNKNOWN,
    })) else {
        STATUS.signal(Status::Error);
        return;
    };

    let _ = join(
        async {
            if runner.run().await.is_err() {
                STATUS.signal(Status::Error);
            }
        },
        async {
            loop {
                STATUS.signal(Status::Advertising);
                match advertise(&mut peripheral, &server).await {
                    Ok(conn) => {
                        STATUS.signal(Status::Connected);
                        echo(&server, &conn).await;
                    }
                    Err(_) => {
                        STATUS.signal(Status::Error);
                        return;
                    }
                }
            }
        },
    )
    .await;
}

async fn advertise<'values, 'server, C: Controller>(
    peripheral: &mut Peripheral<'values, C, DefaultPacketPool>,
    server: &'server Server<'values>,
) -> Result<GattConnection<'values, 'server, DefaultPacketPool>, BleHostError<C::Error>> {
    let mut adv_data = [0; 31];
    let adv_len = AdStructure::encode_slice(
        &[
            AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
            AdStructure::CompleteServiceUuids128(&[SERVICE_UUID.to_le_bytes()]),
        ],
        &mut adv_data[..],
    )?;
    // Flags (3) + a 128-bit UUID (18) + the name (13) is over 31 bytes, so the name goes
    // in the scan response.
    let mut scan_data = [0; 31];
    let scan_len = AdStructure::encode_slice(
        &[AdStructure::CompleteLocalName(b"CPB Counter")],
        &mut scan_data[..],
    )?;
    let advertiser = peripheral
        .advertise(
            &AdvertisementParameters::default(),
            Advertisement::ConnectableScannableUndirected {
                adv_data: &adv_data[..adv_len],
                scan_data: &scan_data[..scan_len],
            },
        )
        .await?;
    let conn = advertiser.accept().await?.with_attribute_server(server)?;
    Ok(conn)
}

/// Echo every `rx` write back as a `tx` notification, until disconnected.
async fn echo<P: PacketPool>(server: &Server<'_>, conn: &GattConnection<'_, '_, P>) {
    let rx = server.gateway.rx.handle;
    let tx = &server.gateway.tx;
    loop {
        match conn.next().await {
            GattConnectionEvent::Disconnected { .. } => return,
            GattConnectionEvent::Gatt { event } => {
                let mut echoed: Option<Vec<u8, VALUE_MAX>> = None;
                if let GattEvent::Write(write) = &event
                    && write.handle() == rx
                {
                    echoed = write.with_data(|_, data| Vec::from_slice(data).ok());
                }
                if let Ok(reply) = event.accept() {
                    reply.send().await;
                }
                if let Some(value) = echoed {
                    STATUS.signal(Status::Echo);
                    let _ = tx.notify(conn, &value, false).await;
                }
            }
            _ => {}
        }
    }
}
