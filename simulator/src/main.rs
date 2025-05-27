use display::prototypes::types::data::Oneof;
use display::prototypes::types::Data;
use display::*;
use embedded_graphics::mono_font::MonoTextStyleBuilder;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use embedded_graphics_simulator::{OutputSettingsBuilder, SimulatorDisplay};
use epd_waveshare::color::Color;
use prost::Message;

use anyhow::anyhow;
// we need this to be able to extend the DisplayBoxed to be able to return the inner simulator
// display to be able to pass that to the window.update function
trait SimulatorDisplayInner {
    fn inner_simulator_display(&self) -> &SimulatorDisplay<Color>;
}

impl SimulatorDisplayInner for DisplayBoxed<SimulatorDisplay<Color>> {
    fn inner_simulator_display(&self) -> &SimulatorDisplay<Color> {
        &self.0
    }
}
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

pub fn main() -> anyhow::Result<()> {
    dioxus_devtools::connect_subsecond();
    let display_raw: SimulatorDisplay<Color> =
        embedded_graphics_simulator::SimulatorDisplay::new(Size::new(296, 128));
    let mut display = DisplayBoxed(Box::new(display_raw));
    let mut window = embedded_graphics_simulator::Window::new(
        "E-Paper Simulator",
        &OutputSettingsBuilder::new()
            .theme(embedded_graphics_simulator::BinaryColorTheme::Default)
            .build(),
    );

    let default_text_style = MonoTextStyleBuilder::new()
        .font(&embedded_graphics::mono_font::ascii::FONT_6X10)
        .text_color(Color::Black)
        .build();
    display.draw_default_display(default_text_style)?;
    window.update(display.inner_simulator_display());
    let (mut socket, response) = tungstenite::connect(format!("ws://localhost:6600/subscribe"))
        .map_err(|err| anyhow!("Error trying to connect to server {:?}", err))?;
    if response.status() != tungstenite::http::StatusCode::SWITCHING_PROTOCOLS {
        log::info!("Error: {:?}", response.status());
        return Err(anyhow!("Error: {:?}", response.status()));
    }

    display.set_connected()?;

    let mut prevs = PrevText::default();
    let mut prev_connections = PrevConnections {
        battery: "0.00".to_string(),
        grid: "0.00".to_string(),
        sun_inverter: "0.00".to_string(),
    };

    let mut flushed = true;
    let mut rescaled = [0.0; 288];
    'outer: loop {
        for event in window.events() {
            if event == embedded_graphics_simulator::SimulatorEvent::Quit {
                println!("Quit event");
                break 'outer;
            }
        }
        match socket.read() {
            Ok(message) => match message {
                tungstenite::Message::Binary(data) => match Data::decode(data) {
                    Ok(ref data_enum) => {
                        // this function is hot patched once it changes, thanks to the subsecond
                        // crate
                        dioxus_devtools::subsecond::call(|| {
                            match render(
                                &mut display,
                                data_enum,
                                &mut prev_connections,
                                &mut prevs,
                                &mut flushed,
                                &mut rescaled,
                            ) {
                                Ok(_) => {}
                                Err(e) => {
                                    eprintln!("Error occured while rendering: {:?}", e);
                                }
                            };
                        });
                    }

                    Err(e) => {
                        eprintln!("Error decoding message: {:?}", e);
                    }
                },
                random_message => {
                    println!("Message: {:?}", random_message);
                }
            },
            Err(e) => {
                eprintln!("Error reading from socket: {:?}", e);
            }
        }
        window.update(display.inner_simulator_display());
    }

    Ok(())
}
fn render(
    display: &mut DisplayBoxed<SimulatorDisplay<Color>>,
    data: &prototypes::types::Data,
    prev_connections: &mut PrevConnections,
    prevs: &mut PrevText,
    flushed: &mut bool,
    rescaled: &mut [f32; 288],
) -> anyhow::Result<()> {
    println!("received data, starting to render");
    let default_text_style = MonoTextStyleBuilder::new()
        .font(&embedded_graphics::mono_font::ascii::FONT_6X10)
        .text_color(Color::Black)
        .build();
    match &data.oneof {
        Some(Oneof::UiData(ui_data)) => {
            display.clear_text()?;
            let house_pow = match &ui_data.gui_house_pow {
                Some(v) => {
                    prevs.house_pow = v.clone();
                    v
                }
                None => &prevs.house_pow,
            };
            let bat_charge = match &ui_data.gui_bat_data_fuel_charge {
                Some(v) => {
                    prevs.bat_charge = v.clone();
                    v
                }
                None => &prevs.bat_charge,
            };
            let inverter_pow = match &ui_data.gui_inverter_power {
                Some(v) => {
                    prevs.inverter_pow = v.clone();
                    v
                }
                None => &prevs.inverter_pow,
            };
            let grid_pow = match &ui_data.gui_grid_pow {
                Some(v) => {
                    prevs.grid_pow = v.clone();
                    v
                }
                None => &prevs.grid_pow,
            };
            let ts = match &ui_data.ts {
                Some(v) => {
                    prevs.ts = v.clone();
                    v
                }
                None => &prevs.ts,
            };
            let bat_power = match &ui_data.gui_bat_data_power {
                Some(v) => {
                    prev_connections.battery = v.to_string();
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
                } else if bat_power.starts_with("-") && bat_power != "-0.00" {
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
                Color::White,
            )?;
            // to the house always active
            display.draw_connections(ConnectionDirection::Top(true))?;

            // we could also just check for changes and clear the arrows
            // individually to avoid this copying and redrawing
            // will do that in the future

            let grid = match &ui_data.gui_grid_pow {
                Some(v) => {
                    prev_connections.grid = v.clone();
                    v
                }
                None => &prev_connections.grid,
            };
            let sun_inverter = match &ui_data.gui_inverter_power {
                Some(v) => {
                    prev_connections.sun_inverter = v.clone();
                    v
                }
                None => &prev_connections.sun_inverter,
            };

            if bat_power != "0.00" && !bat_power.starts_with("-") {
                display.draw_connections(ConnectionDirection::Bottom(true))?;
            } else if bat_power != "0.00" && bat_power.starts_with("-") {
                display.draw_connections(ConnectionDirection::Bottom(false))?;
            }

            if grid != "0.00" && !grid.starts_with("-") {
                display.draw_connections(ConnectionDirection::Right(false))?;
            } else if grid != "0.00" && grid.starts_with("-") {
                display.draw_connections(ConnectionDirection::Right(true))?;
            }

            if sun_inverter != "0.00" && !sun_inverter.starts_with("-") {
                display.draw_connections(ConnectionDirection::Left(false))?;
            } else if sun_inverter != "-0.00" && sun_inverter.starts_with("-") {
                display.draw_connections(ConnectionDirection::Left(false))?;
            }

            if let Some(weather) = &ui_data.weather {
                if let Some(daily) = &weather.daily {
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
                if let Some(hourly) = &weather.hourly {
                    display.update_weather_data(hourly.clone())?;
                }
            }
            if let Some(total_data) = &ui_data.total_data {
                if total_data.new || *flushed {
                    display.update_total_new(&total_data.consumption, &total_data.generated)?;
                }
            }
        }
        Some(Oneof::Prediction(prediction)) => {
            println!("got prediction: {:?}", prediction);
            if prediction.prediction.len() != 288 {
                return Ok(());
            }
            // rescaling the values. over the wire the values were encoded using i32
            // instead of f32. since the values are in the range of 0-12 this saves
            // stream bandwidth
            for (i, v) in prediction.prediction.iter().enumerate() {
                rescaled[i] = *v as f32 / 1000.0;
            }
            display.update_chart(rescaled)?;
        }
        None => {
            eprintln!("Data that was received is None")
        }
    }
    Ok(())
}
