//! The BLE link to the Chrome gateway: radio set-up, the GATT service, and framing.
//!
//! The rest of the firmware sees two channels: [`OUTGOING`] takes [`ToGateway`]
//! messages, and [`INCOMING`] yields [`LinkEvent`]s (link up, link down, and decoded
//! [`ToDevice`] messages). The link is "up" once the gateway has subscribed to `tx`
//! notifications; before that a notification would be dropped silently, so nothing may
//! be sent.

use alloc::vec::Vec;

use cpb_protocol::{Reassembler, SERVICE_UUID_U128, ToDevice, ToGateway, chunks, decode, encode};
use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_futures::select::{Either, select};
use embassy_nrf::mode::Async;
use embassy_nrf::peripherals::{
    PPI_CH17, PPI_CH18, PPI_CH19, PPI_CH20, PPI_CH21, PPI_CH22, PPI_CH23, PPI_CH24, PPI_CH25,
    PPI_CH26, PPI_CH27, PPI_CH28, PPI_CH29, PPI_CH30, PPI_CH31, RNG, RTC0, TEMP, TIMER0,
};
use embassy_nrf::{Peri, bind_interrupts, rng};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use heapless::Vec as HVec;
use nrf_sdc::mpsl::MultiprotocolServiceLayer;
use nrf_sdc::{self as sdc, SoftdeviceController, mpsl};
use static_cell::StaticCell;
use trouble_host::prelude::*;

bind_interrupts!(struct Irqs {
    RNG => rng::InterruptHandler<RNG>;
    EGU0_SWI0 => mpsl::LowPrioInterruptHandler;
    CLOCK_POWER => mpsl::ClockInterruptHandler;
    RADIO => mpsl::HighPrioInterruptHandler;
    TIMER0 => mpsl::HighPrioInterruptHandler;
    RTC0 => mpsl::HighPrioInterruptHandler;
});

/// The largest value one ATT write or notification can carry with the default packet pool.
const VALUE_MAX: usize = 244;

/// Device → gateway messages, waiting to be sent.
pub static OUTGOING: Channel<CriticalSectionRawMutex, ToGateway, 16> = Channel::new();
/// What happened on the link.
pub static INCOMING: Channel<CriticalSectionRawMutex, LinkEvent, 16> = Channel::new();

#[derive(Debug)]
pub enum LinkEvent {
    /// Advertising, waiting for the gateway.
    Advertising,
    /// The gateway is connected and subscribed: messages can be sent.
    Up,
    /// The gateway went away. Messages sent and not yet answered never will be.
    Down,
    Message(ToDevice),
    /// The radio stack failed; the link is gone for good.
    Failed,
}

#[gatt_server]
struct Server {
    gateway: GatewayService,
}

#[gatt_service(uuid = "744aa621-b4b9-4751-91c0-a43b54bb1544")]
struct GatewayService {
    /// Device -> gateway.
    #[characteristic(uuid = "ad229cac-4e64-4208-b76b-ed536a1e47c5", notify)]
    tx: HVec<u8, VALUE_MAX>,
    /// Gateway -> device.
    #[characteristic(uuid = "cb4206ec-fe9c-4fea-ae65-2e7c5b56515b", write_without_response, write)]
    rx: HVec<u8, VALUE_MAX>,
}

/// The peripherals MPSL and the SoftDevice Controller take over.
pub struct Radio {
    pub rtc0: Peri<'static, RTC0>,
    pub timer0: Peri<'static, TIMER0>,
    pub temp: Peri<'static, TEMP>,
    pub rng: Peri<'static, RNG>,
    /// PPI channels 17–31, in order.
    #[allow(clippy::type_complexity)]
    pub ppi: (
        Peri<'static, PPI_CH17>,
        Peri<'static, PPI_CH18>,
        Peri<'static, PPI_CH19>,
        Peri<'static, PPI_CH20>,
        Peri<'static, PPI_CH21>,
        Peri<'static, PPI_CH22>,
        Peri<'static, PPI_CH23>,
        Peri<'static, PPI_CH24>,
        Peri<'static, PPI_CH25>,
        Peri<'static, PPI_CH26>,
        Peri<'static, PPI_CH27>,
        Peri<'static, PPI_CH28>,
        Peri<'static, PPI_CH29>,
        Peri<'static, PPI_CH30>,
        Peri<'static, PPI_CH31>,
    ),
}

/// Take the peripherals MPSL and the SoftDevice Controller own out of `$p`.
#[macro_export]
macro_rules! radio {
    ($p:ident) => {
        $crate::link::Radio {
            rtc0: $p.RTC0,
            timer0: $p.TIMER0,
            temp: $p.TEMP,
            rng: $p.RNG,
            ppi: (
                $p.PPI_CH17, $p.PPI_CH18, $p.PPI_CH19, $p.PPI_CH20, $p.PPI_CH21, $p.PPI_CH22,
                $p.PPI_CH23, $p.PPI_CH24, $p.PPI_CH25, $p.PPI_CH26, $p.PPI_CH27, $p.PPI_CH28,
                $p.PPI_CH29, $p.PPI_CH30, $p.PPI_CH31,
            ),
        }
    };
}

#[embassy_executor::task]
async fn mpsl_task(mpsl: &'static MultiprotocolServiceLayer<'static>) -> ! {
    mpsl.run().await
}

/// Start MPSL and build the SoftDevice Controller. `None` if the radio stack refuses.
pub fn controller(spawner: Spawner, radio: Radio) -> Option<SoftdeviceController<'static>> {
    let (c17, c18, c19, c20, c21, c22, c23, c24, c25, c26, c27, c28, c29, c30, c31) = radio.ppi;
    let mpsl_p = mpsl::Peripherals::new(radio.rtc0, radio.timer0, radio.temp, c19, c30, c31);
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
    let mpsl = MPSL.init(MultiprotocolServiceLayer::new(mpsl_p, Irqs, lfclk_cfg).ok()?);
    spawner.spawn(mpsl_task(mpsl).ok()?);

    let sdc_p = sdc::Peripherals::new(
        c17, c18, c20, c21, c22, c23, c24, c25, c26, c27, c28, c29,
    );
    static RNG_CELL: StaticCell<rng::Rng<'static, Async>> = StaticCell::new();
    let rng = RNG_CELL.init(rng::Rng::new(radio.rng, Irqs));
    static MEM: StaticCell<sdc::Mem<4720>> = StaticCell::new();
    let mem = MEM.init(sdc::Mem::new());

    sdc::Builder::new()
        .ok()?
        .support_adv()
        .support_peripheral()
        .peripheral_count(1)
        .ok()?
        .buffer_cfg(
            DefaultPacketPool::MTU as u16,
            DefaultPacketPool::MTU as u16,
            3,
            3,
        )
        .ok()?
        .build(sdc_p, rng, mpsl, mem)
        .ok()
}

/// Run the link forever: advertise, serve one gateway at a time, repeat.
pub async fn run<C: Controller>(controller: C) {
    // A fixed static random address (top two bits set), so Chrome recognises the device
    // across resets and can reconnect without the chooser.
    let address = Address::random([0x5c, 0x0c, 0x4e, 0x75, 0x0b, 0xc3]);
    let mut resources: HostResources<DefaultPacketPool, 1, 2> = HostResources::new();
    let stack = trouble_host::new(controller, &mut resources)
        .set_random_address(address)
        .build();
    let mut runner = stack.runner();
    let mut peripheral = stack.peripheral();

    let Ok(server) = Server::new_with_config(GapConfig::Peripheral(PeripheralConfig {
        name: cpb_protocol::DEVICE_NAME,
        appearance: &appearance::UNKNOWN,
    })) else {
        INCOMING.send(LinkEvent::Failed).await;
        return;
    };

    join(
        async {
            if runner.run().await.is_err() {
                INCOMING.send(LinkEvent::Failed).await;
            }
        },
        async {
            loop {
                INCOMING.send(LinkEvent::Advertising).await;
                let Ok(conn) = advertise(&mut peripheral, &server).await else {
                    INCOMING.send(LinkEvent::Failed).await;
                    return;
                };
                serve(&server, &conn).await;
                OUTGOING.clear();
                INCOMING.send(LinkEvent::Down).await;
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
            AdStructure::CompleteServiceUuids128(&[SERVICE_UUID_U128.to_le_bytes()]),
        ],
        &mut adv_data[..],
    )?;
    // Flags (3) + a 128-bit UUID (18) + the name (13) is over 31 bytes, so the name goes
    // in the scan response.
    let mut scan_data = [0; 31];
    let scan_len = AdStructure::encode_slice(
        &[AdStructure::CompleteLocalName(cpb_protocol::DEVICE_NAME.as_bytes())],
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
    Ok(advertiser.accept().await?.with_attribute_server(server)?)
}

/// Serve one connection until it drops: GATT events in, notifications out.
async fn serve<P: PacketPool>(server: &Server<'_>, conn: &GattConnection<'_, '_, P>) {
    let rx_handle = server.gateway.rx.handle;
    let tx = &server.gateway.tx;
    let cccd = tx.cccd_handle;
    let mut subscribed = false;
    let mut reassembler = Reassembler::new();

    loop {
        // Only take outgoing messages once the gateway can receive them.
        let outgoing = async {
            if subscribed {
                OUTGOING.receive().await
            } else {
                core::future::pending().await
            }
        };
        match select(conn.next(), outgoing).await {
            Either::First(GattConnectionEvent::Disconnected { .. }) => return,
            Either::First(GattConnectionEvent::Gatt { event }) => {
                let mut data: Option<Vec<u8>> = None;
                let mut cccd_written: Option<bool> = None;
                if let GattEvent::Write(write) = &event {
                    if write.handle() == rx_handle {
                        data = Some(write.with_data(|_, bytes| bytes.to_vec()));
                    } else if Some(write.handle()) == cccd {
                        cccd_written = Some(write.with_data(|_, bytes| {
                            bytes.first().is_some_and(|flags| flags & 1 != 0)
                        }));
                    }
                }
                if let Ok(reply) = event.accept() {
                    reply.send().await;
                }

                if let Some(notify) = cccd_written {
                    if notify && !subscribed {
                        INCOMING.send(LinkEvent::Up).await;
                    }
                    subscribed = notify;
                }
                if let Some(data) = data {
                    match reassembler.push(&data) {
                        Ok(bodies) => {
                            for body in bodies {
                                if let Ok(message) = decode::<ToDevice>(&body) {
                                    INCOMING.send(LinkEvent::Message(message)).await;
                                }
                            }
                        }
                        Err(_) => reassembler.reset(),
                    }
                }
            }
            Either::First(_) => {}
            Either::Second(message) => {
                let frame = encode(&message);
                let size = usize::from(conn.raw().att_mtu().saturating_sub(3)).clamp(20, VALUE_MAX);
                for chunk in chunks(&frame, size) {
                    if tx.notify_raw(conn, chunk, false).await.is_err() {
                        break;
                    }
                }
            }
        }
    }
}
