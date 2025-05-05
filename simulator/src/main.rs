use anyhow::*;
use display::*;
use embedded_graphics::mono_font::MonoTextStyleBuilder;
use embedded_graphics::prelude::*;
use embedded_graphics_simulator::{OutputSettingsBuilder, SimulatorDisplay};
use epd_waveshare::color::Color;

// we need this to be able to extend the DisplayBoxed to be able to return the inner simulator
// display
trait SimulatorDisplayInner {
    fn inner_simulator_display(&self) -> &SimulatorDisplay<Color>;
}

impl SimulatorDisplayInner for DisplayBoxed<SimulatorDisplay<Color>> {
    fn inner_simulator_display(&self) -> &SimulatorDisplay<Color> {
        &self.0
    }
}

pub fn main() -> anyhow::Result<()> {
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
    'outer: loop {
        for event in window.events() {
            if event == embedded_graphics_simulator::SimulatorEvent::Quit {
                println!("Quit event");
                break 'outer;
            }
        }
    }

    Ok(())
}
