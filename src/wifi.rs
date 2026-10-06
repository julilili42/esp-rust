use embassy_executor::Spawner;
use embassy_net::{Ipv4Address, Ipv4Cidr, Runner, Stack, StackResources, StaticConfigV4};
use embassy_time::{Duration, Timer};
use esp_alloc as _;
use esp_backtrace as _;
use esp_hal::{ram, rng::Rng};
use esp_println::println;
use esp_radio::wifi::{
    AuthenticationMethodConfig, Config, ControllerConfig, Interface, WifiController, WifiError,
    ap::AccessPointConfig, scan::ScanConfig, sta::StationConfig,
};

pub const ACCESS_POINT: bool = match option_env!("WIFI_MODE") {
    Some(mode) => match mode.as_bytes() {
        b"ap" => true,
        b"station" => false,
        _ => panic!("WIFI_MODE must be 'ap' or 'station'"),
    },
    None => false,
};

macro_rules! mk_static {
    ($t:ty,$val:expr) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        #[deny(unused_attributes)]
        let x = STATIC_CELL.uninit().write(($val));
        x
    }};
}

pub async fn connect(
    spawner: Spawner,
    device: esp_hal::peripherals::WIFI<'static>,
    ssid: &str,
    password: &str,
) -> Result<Stack<'static>, WifiError> {
    esp_alloc::heap_allocator!(#[ram(reclaimed)] size: 64 * 1024);
    esp_alloc::heap_allocator!(size: 36 * 1024);

    let authentication = AuthenticationMethodConfig::Wpa2Personal(password.try_into()?);
    let (wifi_config, wifi_interface, config) = if ACCESS_POINT {
        (
            Config::AccessPoint(
                AccessPointConfig::default()
                    .with_ssid(ssid.try_into()?)
                    .with_authentication(authentication),
            ),
            Interface::access_point(),
            embassy_net::Config::ipv4_static(StaticConfigV4 {
                address: Ipv4Cidr::new(Ipv4Address::new(192, 168, 4, 1), 24),
                gateway: None,
                dns_servers: Default::default(),
            }),
        )
    } else {
        (
            Config::Station(
                StationConfig::default()
                    .with_ssid(ssid.try_into()?)
                    .with_authentication(authentication),
            ),
            Interface::station(),
            embassy_net::Config::dhcpv4(Default::default()),
        )
    };

    println!("Starting wifi");
    let mut controller = esp_radio::wifi::WifiController::new(
        device,
        ControllerConfig::default().with_initial_config(wifi_config),
    )?;
    println!("Wifi configured");

    let rng = Rng::new();
    let seed = (rng.random() as u64) << 32 | rng.random() as u64;

    // Init network stack
    let (stack, runner) = embassy_net::new(
        wifi_interface,
        config,
        mk_static!(StackResources<3>, StackResources::<3>::new()),
        seed,
    );

    if ACCESS_POINT {
        println!("Access point '{}' ready", ssid);
    } else {
        println!("Scan");
        let scan_config = ScanConfig::default().with_max(10);
        let result = controller.scan_async(&scan_config).await?;
        for ap in result {
            println!("{:?}", ap);
        }
    }

    spawner.spawn(connection(controller).map_err(|_| WifiError::Other)?);
    spawner.spawn(net_task(runner).map_err(|_| WifiError::Other)?);

    stack.wait_config_up().await;

    if let Some(config) = stack.config_v4() {
        println!("Got IP: {}", config.address);
    }

    Ok(stack)
}

#[embassy_executor::task]
async fn connection(mut controller: WifiController<'static>) {
    if ACCESS_POINT {
        loop {
            let event = controller
                .wait_for_access_point_connected_event_async()
                .await;
            println!("Access point event: {:?}", event);
        }
    }

    println!("start connection task");

    loop {
        println!("About to connect...");

        match controller.connect_async().await {
            Ok(info) => {
                println!("Wifi connected to {:?}", info.ssid);

                // wait until we're no longer connected
                let info = controller.wait_for_disconnect_async().await.ok();
                println!("Disconnected: {:?}", info);
            }
            Err(e) => {
                println!("Failed to connect to wifi: {e:?}");
            }
        }

        Timer::after(Duration::from_millis(5000)).await
    }
}

#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, Interface>) {
    runner.run().await
}
