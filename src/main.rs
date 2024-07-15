pub mod client;
pub mod display;
pub mod types;
pub mod wifi;

use client::{convert_connect_error, create_tcp_conn_and_client};
use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::mono_font::MonoTextStyleBuilder;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::Point;
use embedded_graphics::text::{Text, TextStyleBuilder};
use embedded_graphics::Drawable;
use embedded_websocket::framer::{Framer, ReadResult};
use epd_waveshare::prelude::WaveshareDisplay;
use epd_waveshare::prelude::*;
use esp_idf_hal::delay;
use std::time::Duration;

use anyhow::{anyhow, Result};
use esp_idf_hal::peripherals::Peripherals;

use crate::display::init_display;
use crate::wifi::connect_to_wifi;

fn main() -> Result<()> {
    let wifi_password = option_env!("WIFI_PASS").ok_or(anyhow!("wifi_pass not set"))?;
    let wifi_ssid = option_env!("WIFI_SSID").ok_or(anyhow!("wifi_ssid not set"))?;
    let server_addr = option_env!("SERVER_ADDR").ok_or(anyhow!("server_addr not set"))?;
    esp_idf_svc::sys::link_patches();

    esp_idf_svc::log::EspLogger::initialize_default();

    // get peripherals
    let peripherals = Peripherals::take()?;

    // connecting to wifi
    let mut _wifi = connect_to_wifi(peripherals.modem, wifi_ssid, wifi_password)?;

    // setting up display
    let (mut display, mut epd, mut driver) = init_display(
        peripherals.spi2,
        peripherals.pins.gpio48,
        peripherals.pins.gpio38,
        peripherals.pins.gpio21,
        peripherals.pins.gpio10,
        peripherals.pins.gpio18,
        peripherals.pins.gpio17,
    )?;
    log::info!("Got the display");
    let default_text_style = MonoTextStyleBuilder::new()
        .font(&embedded_graphics::mono_font::ascii::FONT_6X10)
        .text_color(BinaryColor::On)
        .build();
    let _text_style_baseline = TextStyleBuilder::new()
        .baseline(embedded_graphics::text::Baseline::Top)
        .build();

    let mut retries = 0;
    loop {
        // Clear the display from any remainders
        epd.clear_frame(&mut driver, &mut delay::Ets)?;
        display.clear(BinaryColor::Off)?;

        // draw the ui with default values
        display.draw_default_display(default_text_style)?;
        epd.update_and_display_frame(&mut driver, display.buffer(), &mut delay::Ets)?;
        epd.update_old_frame(&mut driver, display.buffer(), &mut delay::Ets)?;

        if retries > 5 {
            break;
        }
        log::info!("Retry: {}", retries);
        let mut read_cursor = 0;
        // we dont need this after the intial request was send
        let mut write_buf = [0; 500];
        let mut read_buf = [0; 500];

        let mut frame_buf = [0; 2000];
        log::info!("Starting tcp conn to addr: {} ", server_addr);
        let (mut stream, options, mut client) = create_tcp_conn_and_client(server_addr)?;
        log::info!("tcp conn success");
        let mut framer = Framer::new(&mut read_buf, &mut read_cursor, &mut write_buf, &mut client);

        match framer.connect(&mut stream, &options) {
            Ok(_) => (),
            Err(e) => {
                log::info!("Error: {}", convert_connect_error(e));
                retries += 1;

                continue;
            }
        };

        log::info!("Connected to websocket");
        display.set_connected()?;
        epd.update_new_frame(&mut driver, display.buffer(), &mut delay::Ets)?;
        epd.display_new_frame(&mut driver, &mut delay::Ets)?;
        epd.update_old_frame(&mut driver, display.buffer(), &mut delay::Ets)?;

        // start time
        let mut curr_time = std::time::SystemTime::now();

        let mut flushed = true;
        'inner: loop {
            match framer.read(&mut stream, &mut frame_buf) {
                Ok(read_res) => match read_res {
                    ReadResult::Binary(_) => continue,
                    ReadResult::Pong(_) => continue,
                    ReadResult::Closed => continue,
                    ReadResult::Text(t) => {
                        let time_now = std::time::SystemTime::now();
                        let since = time_now.duration_since(curr_time)?;
                        if since > Duration::from_secs(120) {
                            display.clear_buffer(Color::White);
                            display.draw_default_display(default_text_style)?;
                            display.set_connected()?;
                            epd.update_and_display_frame(
                                &mut driver,
                                display.buffer(),
                                &mut delay::Ets,
                            )?;
                            epd.update_old_frame(&mut driver, display.buffer(), &mut delay::Ets)?;
                            curr_time = time_now;
                            flushed = true;
                        }
                        log::info!("Got a message {}", t);

                        match serde_json::from_str::<types::NewUiStruct>(t) {
                            Ok(json_values) => {
                                display.clear_text()?;
                                display.draw_text(
                                    default_text_style,
                                    json_values.gui_house_pow,
                                    &match json_values.gui_bat_data_power.contains("-") {
                                        // meaning the battery is being charged
                                        false => {
                                            format!("+{}", json_values.gui_bat_data_fuel_charge)
                                        }
                                        // meaning battery is being discharged
                                        true => {
                                            format!("-{}", json_values.gui_bat_data_fuel_charge)
                                        }
                                    },
                                    json_values.gui_inverter_power,
                                    &match json_values.gui_grid_pow.starts_with("-") {
                                        true => format!("{}", json_values.gui_grid_pow),
                                        false => format!("+{}", json_values.gui_grid_pow),
                                    },
                                    json_values.ts,
                                )?;

                                // to the house always active
                                display
                                    .draw_connections(display::ConnectionDirection::Top(true))?;

                                // will rework the conditions in the future

                                if json_values.gui_bat_data_power != "0.00"
                                    && !json_values.gui_bat_data_power.starts_with("-")
                                {
                                    // to the battery since it is being discharged
                                    display.draw_connections(
                                        display::ConnectionDirection::Bottom(false),
                                    )?;
                                }
                                if json_values.gui_bat_data_power.starts_with("-")
                                    && json_values.gui_bat_data_power != "0.00"
                                {
                                    display.draw_connections(
                                        display::ConnectionDirection::Bottom(false),
                                    )?
                                } else if !json_values.gui_bat_data_power.starts_with("-")
                                    && json_values.gui_bat_data_power != "0.00"
                                {
                                    // to the battery since it is being charged
                                    display.draw_connections(
                                        display::ConnectionDirection::Bottom(true),
                                    )?;
                                }

                                // power send to the grid
                                if json_values.gui_grid_pow.starts_with("-")
                                    && json_values.gui_grid_pow != "-0.00"
                                {
                                    display.draw_connections(
                                        display::ConnectionDirection::Right(true),
                                    )?;
                                } else if !json_values.gui_grid_pow.starts_with("-")
                                    && json_values.gui_grid_pow != "0.00"
                                {
                                    // power taken from the grid
                                    display.draw_connections(
                                        display::ConnectionDirection::Right(false),
                                    )?;
                                }

                                if json_values.gui_inverter_power != "0.00"
                                    && !json_values.gui_inverter_power.starts_with("-")
                                {
                                    display.draw_connections(
                                        display::ConnectionDirection::Left(false),
                                    )?;
                                }

                                if json_values.total_data.new || flushed {
                                    display.update_total_display(
                                        json_values.total_data.consumption,
                                        json_values.total_data.generated,
                                    )?;
                                    // this only needs to be updated every hour
                                    let sunrise = json_values
                                        .weather
                                        .daily
                                        .sunrise
                                        .get(0)
                                        .ok_or(anyhow!("error value not present"))?;
                                    let sunset = json_values
                                        .weather
                                        .daily
                                        .sunset
                                        .get(0)
                                        .ok_or(anyhow!("error value not present"))?;
                                    display.update_sun_data(sunrise, sunset)?;
                                    display.update_weather_data(json_values.weather.hourly)?;
                                    flushed = false;
                                }

                                epd.update_new_frame(
                                    &mut driver,
                                    display.buffer(),
                                    &mut delay::Ets,
                                )?;
                                epd.display_new_frame(&mut driver, &mut delay::Ets)?;
                                epd.update_old_frame(
                                    &mut driver,
                                    display.buffer(),
                                    &mut delay::Ets,
                                )?;

                                continue;
                            }
                            Err(e) => {
                                log::info!("An error occured: {:?} Message: {:?}  ", e, t);
                                display.clear(BinaryColor::Off)?;
                                display.display_error_message(
                                    "Error decoding message!",
                                    default_text_style,
                                )?;
                                epd.update_and_display_frame(
                                    &mut driver,
                                    display.buffer(),
                                    &mut delay::Ets,
                                )?;
                                epd.update_old_frame(
                                    &mut driver,
                                    display.buffer(),
                                    &mut delay::Ets,
                                )?;
                                // sleep for 12s to reduce power consumption
                                // still todo

                                continue;
                            }
                        }
                    }
                },
                Err(e) => {
                    println!("Error :{:?}", e);
                    break 'inner;
                }
            }
        }
        retries += 1;
        display.clear_buffer(Color::White);
        Text::new(
            &format!("Disconnected from Websocket! Retry: {}", retries),
            Point::new(45, 40),
            default_text_style,
        )
        .draw(&mut display)?;
        epd.update_and_display_frame(&mut driver, display.buffer(), &mut delay::Ets)?;

        continue;
    }

    display.clear_buffer(Color::White);
    Text::new(
        "Disconnected from Websocket!",
        Point::new(60, 40),
        default_text_style,
    )
    .draw(&mut display)?;
    Text::new(
        "Manual restart necessary",
        Point::new(60, 50),
        default_text_style,
    )
    .draw(&mut display)?;
    epd.update_and_display_frame(&mut driver, display.buffer(), &mut delay::Ets)?;

    Ok(())
}
