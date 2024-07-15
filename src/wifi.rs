use esp_idf_hal::modem::Modem;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::{ClientConfiguration, Configuration, EspWifi};

use esp_idf_svc::wifi::BlockingWifi;
pub fn connect_to_wifi<'a>(
    modem: Modem,
    ssid: &'a str,
    pass: &'a str,
) -> anyhow::Result<BlockingWifi<EspWifi<'a>>> {
    let esp_sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    let esp_wifi = EspWifi::new(modem, esp_sys_loop.clone(), Some(nvs))?;
    let mut blocking_wifi = BlockingWifi::wrap(esp_wifi, esp_sys_loop)?;
    blocking_wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: ssid.into(),
        bssid: None,
        auth_method: esp_idf_svc::wifi::AuthMethod::WPA2WPA3Personal,
        password: pass.into(),
        channel: None,
    }))?;
    blocking_wifi.start()?;
    blocking_wifi.connect()?;
    blocking_wifi.wait_netif_up()?;
    while !blocking_wifi.is_connected().unwrap() {}
    return Ok(blocking_wifi);
}
