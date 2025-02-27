pub mod client;
pub mod display;
pub mod types;
pub mod wifi;
// proto defitions
pub mod prototypes {
    pub mod types {
        include!(concat!(env!("OUT_DIR"), "/prototypes.types.rs"));
    }
}
use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::mono_font::MonoTextStyleBuilder;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::Point;
use embedded_graphics::prelude::Size;
use embedded_graphics::primitives::Rectangle;
use embedded_graphics::text::{Text, TextStyleBuilder};
use embedded_graphics::Drawable;
use epd_waveshare::prelude::WaveshareDisplay;
use epd_waveshare::prelude::*;
use esp_idf_hal::delay;
use prost::Message;
use std::time::Duration;

use anyhow::anyhow;
use esp_idf_hal::peripherals::Peripherals;

use crate::display::init_display;
use crate::wifi::connect_to_wifi;

#[derive(Debug)]
struct PrevText {
    house_pow: String,
    bat_charge: String,
    inverter_pow: String,
    grid_pow: String,
    ts: String,
}
impl Default for PrevText {
    fn default() -> Self {
        PrevText {
            house_pow: "0.00".to_string(),
            bat_charge: "0.00".to_string(),
            inverter_pow: "0.00".to_string(),
            grid_pow: "0.00".to_string(),
            ts: "0:00".to_string(),
        }
    }
}
#[derive(Debug)]
struct PrevConnections {
    battery: String,
    grid: String,
    sun_inverter: String,
}

fn main() -> anyhow::Result<()> {
    let wifi_password = option_env!("WIFI_PASS").ok_or(anyhow!("wifi_pass not set"))?;
    let wifi_ssid = option_env!("WIFI_SSID").ok_or(anyhow!("wifi_ssid not set"))?;
    let server_addr = option_env!("SERVER_ADDR").ok_or(anyhow!("server_addr not set"))?;
    esp_idf_svc::sys::link_patches();

    esp_idf_svc::log::EspLogger::initialize_default();

    // get peripherals
    let peripherals = Peripherals::take()?;

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

    // connecting to wifi
    display.draw_status_message("Connecting to Wifi")?;
    epd.update_and_display_frame(&mut driver, display.buffer(), &mut delay::Ets)?;
    epd.update_old_frame(&mut driver, display.buffer(), &mut delay::Ets)?;

    let mut _wifi = connect_to_wifi(peripherals.modem, wifi_ssid, wifi_password)?;

    // update display
    display.clear_status_message()?;
    display.draw_status_message("Wifi success")?;
    epd.update_new_frame(&mut driver, display.buffer(), &mut delay::Ets)?;
    epd.display_new_frame(&mut driver, &mut delay::Ets)?;

    let default_text_style = MonoTextStyleBuilder::new()
        .font(&embedded_graphics::mono_font::ascii::FONT_6X10)
        .text_color(BinaryColor::On)
        .build();
    let _text_style_baseline = TextStyleBuilder::new()
        .baseline(embedded_graphics::text::Baseline::Top)
        .build();

    let mut retries = 0;
    'outer: loop {
        log::info!("Retry: {}", retries);
        if retries > 5 {
            break;
        }
        // Clear the display from any remainders
        display.clear_status_message()?;
        epd.update_new_frame(&mut driver, display.buffer(), &mut delay::Ets)?;
        epd.display_new_frame(&mut driver, &mut delay::Ets)?;

        // drawing default interface
        display.draw_default_display(default_text_style)?;
        epd.update_and_display_frame(&mut driver, display.buffer(), &mut delay::Ets)?;
        epd.update_old_frame(&mut driver, display.buffer(), &mut delay::Ets)?;

        let (mut socket, response) =
            tungstenite::connect(format!("ws://{}/subscribe", server_addr))?;
        if response.status() != tungstenite::http::StatusCode::SWITCHING_PROTOCOLS {
            log::info!("Error: {:?}", response.status());
            retries += 1;
            continue;
        }
        log::info!("Connected to websocket");
        display.set_connected()?;
        epd.update_new_frame(&mut driver, display.buffer(), &mut delay::Ets)?;
        epd.display_new_frame(&mut driver, &mut delay::Ets)?;
        epd.update_old_frame(&mut driver, display.buffer(), &mut delay::Ets)?;

        //start time
        let mut curr_time = std::time::SystemTime::now();

        // we store the prediction results buffer and preallocate
        let mut rescaled = [0.0; 288];

        // we make sure that we repaint fully, if it has been fully flushed
        let mut flushed = true;

        // if there was an error before we need to repaint the default display
        let mut prev_error = false;

        let mut prevs = PrevText::default();
        let mut prev_connections = PrevConnections {
            battery: "0.00".to_string(),
            grid: "0.00".to_string(),
            sun_inverter: "0.00".to_string(),
        };

        let mut repaints = 0;
        'inner: loop {
            if retries > 5 {
                break 'outer;
            }

            match socket.read() {
                Ok(message) => match message {
                    tungstenite::Message::Text(t) => {
                        println!("got a text message: {:?}", t);
                        continue;
                    }
                    tungstenite::Message::Binary(b) => {
                        match prototypes::types::Data::decode(b) {
                            Ok(data_enum) => match data_enum.oneof {
                                Some(prototypes::types::data::Oneof::UiData(data)) => {
                                    println!("got ui data: {:?}", data);
                                    let time_now = std::time::SystemTime::now();
                                    let since = time_now.duration_since(curr_time)?;
                                    // every 2 mins we do a full repaint, refresh of the display to clean up
                                    // small imperfections in the pixles that occur due to quick refreshes
                                    if since > Duration::from_secs(120) {
                                        // the issue with disapperaing weather information was:
                                        // the displays buffer is cleared( which
                                        // contains the weather information and then the prev
                                        // buffer is painted ) but when the next repaint comes
                                        // around the prev buffer no longer contains the weather
                                        // information so we lose that information
                                        println!("full repaint {}", repaints);
                                        // we have to copy the displays buffer to be able to
                                        // repaint the same display after a full fresh up
                                        let last_buff = display.buffer();
                                        let prev_buffer = last_buff.to_vec();

                                        //then we clear the displys buffer + paint the default
                                        //structures
                                        display.clear_buffer(Color::White);
                                        display.draw_default_display(default_text_style)?;

                                        // now we reset the old buffer
                                        display.set_buf(&prev_buffer)?;
                                        // in the case that the screen was an error screen before we need to
                                        // repaint the default display
                                        if prev_error {
                                            println!("repainting default display");
                                            epd.update_and_display_frame(
                                                &mut driver,
                                                display.buffer(),
                                                &mut delay::Ets,
                                            )?;
                                        } else {
                                            epd.update_and_display_frame(
                                                &mut driver,
                                                prev_buffer.as_slice(),
                                                &mut delay::Ets,
                                            )?;
                                        }

                                        repaints += 1;

                                        epd.update_old_frame(
                                            &mut driver,
                                            display.buffer(),
                                            &mut delay::Ets,
                                        )?;
                                        curr_time = time_now;
                                        flushed = true;
                                    }

                                    display.clear_text()?;

                                    // very verbose incoming
                                    // we could solve this by creating more granular drawing
                                    // functionality, so we only have to draw the smallest parts
                                    // that change, but this will be extremely verbose and might
                                    // not really matter performance wise

                                    let house_pow = match &data.gui_house_pow {
                                        Some(v) => {
                                            prevs.house_pow = v.clone();
                                            v
                                        }
                                        None => &prevs.house_pow,
                                    };
                                    let bat_charge = match &data.gui_bat_data_fuel_charge {
                                        Some(v) => {
                                            prevs.bat_charge = v.clone();
                                            v
                                        }
                                        None => &prevs.bat_charge,
                                    };
                                    let inverter_pow = match &data.gui_inverter_power {
                                        Some(v) => {
                                            prevs.inverter_pow = v.clone();
                                            v
                                        }
                                        None => &prevs.inverter_pow,
                                    };
                                    let grid_pow = match &data.gui_grid_pow {
                                        Some(v) => {
                                            prevs.grid_pow = v.clone();
                                            v
                                        }
                                        None => &prevs.grid_pow,
                                    };
                                    let ts = match &data.ts {
                                        Some(v) => {
                                            prevs.ts = v.clone();
                                            v
                                        }
                                        None => &prevs.ts,
                                    };
                                    let bat_power = match &data.gui_bat_data_power {
                                        Some(v) => {
                                            prev_connections.battery = v.clone();
                                            v
                                        }
                                        None => prev_connections.battery.as_str(),
                                    };
                                    display.draw_text(
                                        default_text_style,
                                        &house_pow,
                                        &if bat_power != "0.00" && !bat_power.starts_with("-") {
                                            // bat_power is the current going to the battery,
                                            // therefore if non 0 and not starting with a minus we
                                            // are charging
                                            format!("+{}", bat_charge)
                                        } else if bat_power.starts_with("-") && bat_power != "-0.00"
                                        {
                                            // in this case we are discharging
                                            format!("-{}", bat_charge)
                                        } else {
                                            // no current flowing in or out of the battery
                                            format!("{}", bat_charge)
                                        },
                                        &inverter_pow,
                                        &match grid_pow.starts_with("-") {
                                            true => format!("{}", grid_pow),
                                            false => format!("+{}", grid_pow),
                                        },
                                        &ts,
                                    )?;

                                    // clearing the connections
                                    display.fill_solid(
                                        &Rectangle::new(Point::new(54, 43), Size::new(42, 41)),
                                        BinaryColor::Off,
                                    )?;
                                    // to the house always active
                                    display.draw_connections(display::ConnectionDirection::Top(
                                        true,
                                    ))?;

                                    // we could also just check for changes and clear the arrows
                                    // individually to avoid this copying and redrawing
                                    // will do that in the future

                                    let grid = match &data.gui_grid_pow {
                                        Some(v) => {
                                            prev_connections.grid = v.clone();
                                            v
                                        }
                                        None => &prev_connections.grid,
                                    };
                                    let sun_inverter = match &data.gui_inverter_power {
                                        Some(v) => {
                                            prev_connections.sun_inverter = v.clone();
                                            v
                                        }
                                        None => &prev_connections.sun_inverter,
                                    };

                                    if bat_power != "0.00" && !bat_power.starts_with("-") {
                                        display.draw_connections(
                                            display::ConnectionDirection::Bottom(true),
                                        )?;
                                    } else if bat_power != "0.00" && bat_power.starts_with("-") {
                                        display.draw_connections(
                                            display::ConnectionDirection::Bottom(false),
                                        )?;
                                    }

                                    if grid != "0.00" && !grid.starts_with("-") {
                                        display.draw_connections(
                                            display::ConnectionDirection::Right(false),
                                        )?;
                                    } else if grid != "0.00" && grid.starts_with("-") {
                                        display.draw_connections(
                                            display::ConnectionDirection::Right(true),
                                        )?;
                                    }

                                    if sun_inverter != "0.00" && !sun_inverter.starts_with("-") {
                                        display.draw_connections(
                                            display::ConnectionDirection::Left(false),
                                        )?;
                                    } else if sun_inverter != "-0.00"
                                        && sun_inverter.starts_with("-")
                                    {
                                        display.draw_connections(
                                            display::ConnectionDirection::Left(false),
                                        )?;
                                    }

                                    if let Some(weather) = data.weather {
                                        if let Some(daily) = weather.daily {
                                            let sunrise = daily
                                                .sunrise
                                                .first()
                                                .ok_or(anyhow!("missing sunrise values"))?;
                                            let sunset = daily
                                                .sunset
                                                .first()
                                                .ok_or(anyhow!("missing sunset values"))?;
                                            display.update_sun_data(sunrise, sunset)?;
                                        }
                                        if let Some(hourly) = weather.hourly {
                                            display.update_weather_data(hourly)?;
                                        }
                                    }
                                    if let Some(total_data) = data.total_data {
                                        if total_data.new || flushed {
                                            display.update_total_new(
                                                &total_data.consumption,
                                                &total_data.generated,
                                            )?;
                                        }
                                    }
                                    if flushed {
                                        display.update_chart(&rescaled)?;
                                    }

                                    flushed = false;
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
                                    prev_error = false;

                                    continue;
                                }
                                Some(prototypes::types::data::Oneof::Prediction(prediction)) => {
                                    println!("got prediction: {:?}", prediction);
                                    if prediction.prediction.len() != 288 {
                                        continue;
                                    }
                                    // rescaling the values. over the wire the values were encoded using i32
                                    // instead of f32. since the values are in the range of 0-12 this saves
                                    // stream bandwidth
                                    for (i, v) in prediction.prediction.iter().enumerate() {
                                        rescaled[i] = *v as f32 / 1000.0;
                                    }
                                    display.update_chart(&rescaled)?;
                                    epd.update_new_frame(
                                        &mut driver,
                                        display.buffer(),
                                        &mut delay::Ets,
                                    )?;
                                    epd.display_new_frame(&mut driver, &mut delay::Ets)?;
                                    continue;
                                }
                                None => {
                                    println!("no data present");
                                }
                            },
                            Err(e) => {
                                prev_error = true;
                                println!("error decoding data: {:?}", e);
                                continue;
                            }
                        };

                        println!("error parsing the message");
                    }
                    tungstenite::Message::Close(v) => {
                        println!("connection was closed: {:?}", v);
                        break 'inner;
                    }
                    v => {
                        println!("unexpected message: {:?}", v);
                        continue;
                    }
                },
                Err(e) => {
                    println!("error reading from ws: {:?}", e);
                    retries += 1;
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
