use crate::prototypes::types::HourlyNew;
use anyhow::anyhow;
use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::iterator::PixelIteratorExt;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::mono_font::MonoTextStyleBuilder;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::OriginDimensions;
use embedded_graphics::prelude::Point;
use embedded_graphics::prelude::Size;
use embedded_graphics::primitives::*;
use embedded_graphics::text::Text;
use embedded_graphics::Drawable;
use embedded_graphics::Pixel;
use epd_waveshare::epd2in9_v2::Epd2in9;
use epd_waveshare::prelude::Display;
use epd_waveshare::prelude::DisplayRotation;
use epd_waveshare::{prelude::WaveshareDisplay, *};
use esp_idf_hal::delay;
use esp_idf_hal::delay::Ets;
use esp_idf_hal::gpio;
use esp_idf_hal::gpio::Gpio10;
use esp_idf_hal::gpio::Gpio17;
use esp_idf_hal::gpio::Gpio18;
use esp_idf_hal::gpio::Gpio21;
use esp_idf_hal::gpio::Gpio38;
use esp_idf_hal::gpio::Gpio48;
use esp_idf_hal::gpio::Input;
use esp_idf_hal::gpio::Output;
use esp_idf_hal::gpio::PinDriver;
use esp_idf_hal::spi;
use esp_idf_hal::spi::SpiDeviceDriver;
use esp_idf_hal::spi::SpiDriver;
use esp_idf_hal::spi::SPI2;
use esp_idf_hal::units::Hertz;

// this is for the direction power is comming from
enum ArrowDirection {
    Left,
    Right,
    Down,
    Up,
}
// this is for sunset and sunrise
enum SimpleArrowDirection {
    Up,
    Down,
}
pub enum ConnectionDirection {
    Top(bool),
    Right(bool),
    Left(bool),
    Bottom(bool),
}
// trying to avoid stack overflows so we use heap alloc
pub struct DisplayBoxed(Box<epd2in9_v2::Display2in9>);

impl DrawTarget for DisplayBoxed {
    type Color = BinaryColor;
    type Error = core::convert::Infallible;
    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        self.0.clear(color)
    }
    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = embedded_graphics::Pixel<Self::Color>>,
    {
        self.0.draw_iter(pixels)
    }
    fn fill_solid(
        &mut self,
        area: &embedded_graphics::primitives::Rectangle,
        color: Self::Color,
    ) -> Result<(), Self::Error> {
        self.0.fill_solid(area, color)
    }
    fn fill_contiguous<I>(
        &mut self,
        area: &embedded_graphics::primitives::Rectangle,
        colors: I,
    ) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Self::Color>,
    {
        self.0.fill_contiguous(area, colors)
    }
}
impl Display for DisplayBoxed {
    fn buffer(&self) -> &[u8] {
        self.0.buffer()
    }
    fn rotation(&self) -> DisplayRotation {
        self.0.rotation()
    }
    fn draw_helper(
        &mut self,
        width: u32,
        height: u32,
        pixel: embedded_graphics::Pixel<BinaryColor>,
    ) -> Result<(), Self::Error> {
        self.0.draw_helper(width, height, pixel)
    }
    fn clear_buffer(&mut self, background_color: prelude::Color) {
        self.0.clear_buffer(background_color)
    }
    fn set_rotation(&mut self, rotation: DisplayRotation) {
        self.0.set_rotation(rotation)
    }
    fn get_mut_buffer(&mut self) -> &mut [u8] {
        self.0.get_mut_buffer()
    }
}

impl OriginDimensions for DisplayBoxed {
    fn size(&self) -> embedded_graphics::prelude::Size {
        self.0.size()
    }
}

pub fn init_display<'a>(
    spi2: SPI2,
    gpio48: Gpio48,
    gpio38: Gpio38,
    gpio21: Gpio21,
    gpio10: Gpio10,
    gpio18: Gpio18,
    gpio17: Gpio17,
) -> anyhow::Result<(
    DisplayBoxed,
    Epd2in9<
        SpiDeviceDriver<'a, SpiDriver<'a>>,
        PinDriver<'a, Gpio21, Output>,
        PinDriver<'a, Gpio10, Input>,
        PinDriver<'a, Gpio18, Output>,
        PinDriver<'a, Gpio17, Output>,
        Ets,
    >,
    SpiDeviceDriver<'a, SpiDriver<'a>>,
)> {
    let mut driver = spi::SpiDeviceDriver::new_single(
        spi2,
        gpio48,
        gpio38,
        Option::<gpio::AnyIOPin>::None,
        Option::<gpio::AnyOutputPin>::None,
        &spi::SpiDriverConfig::new().dma(spi::Dma::Disabled),
        &spi::SpiConfig::new().baudrate(Hertz::from(26)),
    )?;

    let cs = gpio::PinDriver::output(gpio21)?;

    let busy = gpio::PinDriver::input(gpio10)?;

    let dc = gpio::PinDriver::output(gpio18)?;

    let rst = gpio::PinDriver::output(gpio17)?;

    let epd = match epd2in9_v2::Epd2in9::new(&mut driver, cs, busy, dc, rst, &mut delay::Ets) {
        std::result::Result::Ok(epd) => epd,
        Err(e) => return Err(anyhow::Error::new(e)),
    };

    let display = Box::new(epd2in9_v2::Display2in9::default());
    let mut dis_boxed = DisplayBoxed { 0: display };

    dis_boxed.set_rotation(DisplayRotation::Rotate90);
    dis_boxed.clear(BinaryColor::Off)?;
    return Ok((dis_boxed, epd, driver));
}
#[rustfmt::skip]
static HOUSE_PATTERN: [u8; 270] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0,
    0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0,
    0,0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 
    0,0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 
    0,0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 
    0,0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 
    0,0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 
    0,0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0,
    0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0,
    0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,

    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];
#[rustfmt::skip]
static TEMP:[u8;270]=[
0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
0,0,0,0,0,0,0,0,1,1,0,0,0,0,0,0,0,0,
0,0,0,0,0,0,0,1,0,0,1,0,0,0,0,0,0,0,
0,0,0,0,0,0,0,1,0,0,1,0,0,0,0,0,0,0,
0,0,0,0,0,0,0,1,0,0,1,0,0,0,0,0,0,0,
0,0,0,0,0,0,0,1,0,0,1,0,0,0,0,0,0,0,
0,0,0,0,0,0,0,1,0,0,1,0,0,0,0,0,0,0,
0,0,0,0,0,0,0,1,0,0,1,0,0,0,0,0,0,0,
0,0,0,0,0,0,1,1,0,0,1,1,0,0,0,0,0,0,
0,0,0,0,0,1,0,0,0,0,0,0,1,0,0,0,0,0,
0,0,0,0,0,1,0,0,0,0,0,0,1,0,0,0,0,0,
0,0,0,0,0,1,0,1,1,1,1,0,1,0,0,0,0,0,
0,0,0,0,0,0,1,0,0,0,0,1,0,0,0,0,0,0,
0,0,0,0,0,0,0,1,1,1,1,0,0,0,0,0,0,0,
];
#[rustfmt::skip]
static CLOUD:[u8;270]=[
0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
0,0,0,0,0,0,0,0,0,1,1,1,0,0,0,0,0,0,
0,0,0,0,0,0,0,0,1,1,1,1,1,0,0,0,0,0,
0,0,0,0,0,1,1,1,1,1,1,1,1,1,0,0,0,0,
0,0,0,0,1,1,1,1,1,1,1,1,1,1,0,0,0,0,
0,0,0,1,1,1,1,1,1,1,1,1,1,1,1,0,0,0,
0,0,1,1,1,1,1,1,1,1,1,1,1,1,1,1,0,0,
0,0,1,1,1,1,1,1,1,1,1,1,1,1,1,1,0,0,
0,0,0,0,1,1,1,1,1,1,1,1,1,1,0,0,0,0,
0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
];
#[rustfmt::skip]
static RAINDROPS:[u8;270]=[
0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
0,0,0,0,0,0,0,0,0,0,1,0,0,0,0,0,0,0,
0,0,0,0,0,0,0,1,1,1,1,1,1,1,0,0,0,0,
0,0,0,0,0,0,0,1,1,1,1,1,1,1,0,0,0,0,
0,0,0,1,1,1,1,1,1,1,1,1,1,1,1,0,0,0,
0,0,1,1,1,1,1,1,1,1,1,1,1,1,1,0,0,0,
0,0,1,1,1,1,1,1,1,1,1,1,1,1,1,1,0,0,
0,0,0,1,1,1,1,1,1,1,1,1,1,1,1,1,0,0,
0,0,0,0,1,1,1,1,1,1,1,1,1,1,0,0,0,0,
0,0,0,0,0,1,0,1,1,0,1,1,0,1,0,0,0,0,
0,0,0,0,0,1,1,0,1,1,0,1,1,0,0,0,0,0,
0,0,0,0,0,0,1,1,0,1,1,0,1,0,0,0,0,0,
0,0,0,0,0,0,0,0,0,0,1,0,0,0,0,0,0,0,
0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
];

#[rustfmt::skip]
static LIGHTNING_BOLT_PATTERN: [u8; 270] = [
    0, 0, 0, 0, 0, 0, 0, 1, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 1, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
];
#[rustfmt::skip]
static BATTERY_PATTERN: [u8; 270] = [
    0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
];

#[rustfmt::skip]
static SUN_PATTERN: [u8; 270] = [

    0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 1, 0, 0, 1, 1, 1, 0, 0, 1, 1, 1, 0, 0, 1, 0, 0,
    0, 0, 0, 1, 1, 1, 0, 0, 0, 0, 0, 0, 1, 1, 1, 0, 0, 0,
    0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0,
    0, 1, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 1, 0,
    0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0,
    0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0,
    0, 1, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 1, 0,
    0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0,
    0, 0, 0, 1, 1, 1, 0, 0, 0, 0, 0, 0, 1, 1, 1, 0, 0, 0,
    0, 0, 1, 0, 0, 1, 1, 1, 0, 0, 1, 1, 1, 0, 0, 1, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];
fn max_in_slice(slice: &[f32]) -> Option<f32> {
    slice.iter().fold(None, |max, &x| match max {
        None => Some(x),
        Some(m) => Some(m.max(x)),
    })
}
impl DisplayBoxed {
    pub fn draw_chart(&mut self, data: &[f32]) -> anyhow::Result<()> {
        let desc_text_style = MonoTextStyleBuilder::new()
            .font(&embedded_graphics::mono_font::ascii::FONT_4X6)
            .text_color(BinaryColor::On)
            .build();

        let line_style = PrimitiveStyle::with_stroke(BinaryColor::On, 1);
        Line::new(Point::new(153, 124), Point::new(286, 124))
            .into_styled(line_style)
            .draw(self)?;

        self.fill_solid(
            &Rectangle::new(Point::new(217, 124), Size::new(9, 8)),
            BinaryColor::Off,
        )?;
        self.fill_solid(
            &Rectangle::new(Point::new(149, 121), Size::new(2, 7)),
            BinaryColor::Off,
        )?;
        Text::new("0", Point::new(149, 126), desc_text_style).draw(self)?;
        Text::new("12", Point::new(218, 126), desc_text_style).draw(self)?;
        Text::new("24", Point::new(288, 126), desc_text_style).draw(self)?;

        let max = max_in_slice(&data).ok_or(anyhow!("error finding max value"))?;
        self.fill_solid(
            &Rectangle::new(Point::new(145, 75), Size::new(10, 8)),
            BinaryColor::Off,
        )?;
        if max >= 10.0 {
            Text::new(
                &format!("{:.2} kW", max),
                Point::new(145, 80),
                desc_text_style,
            )
            .draw(self)?;
        } else {
            Text::new(
                &format!("{:.2} kW", max),
                Point::new(143, 80),
                desc_text_style,
            )
            .draw(self)?;
        }
        let mut averaged = data
            .chunks(2)
            .map(|values| {
                let sum: f32 = values.iter().sum();
                sum / values.len() as f32
            })
            .collect::<Vec<f32>>();

        // scaling to size of display
        averaged
            .iter_mut()
            .for_each(|value| *value = (*value / max * 45.0).round());

        let start_x = 151;
        let start_y = 120;
        let points = averaged
            .iter()
            .enumerate()
            .map(|(i, &v)| {
                let x = start_x + i as i32;
                let y = start_y - v as i32;
                Point::new(x, y)
            })
            .collect::<Vec<Point>>();

        Polyline::new(&points).into_styled(line_style).draw(self)?;

        Ok(())
    }
    pub fn new_total(&mut self, house: &str, solar: &str) -> anyhow::Result<()> {
        let desc_text_style = MonoTextStyleBuilder::new()
            .font(&embedded_graphics::mono_font::ascii::FONT_4X6)
            .text_color(BinaryColor::On)
            .build();

        embedded_graphics::primitives::Rectangle::new(Point::new(100, 91), Size::new(45, 38))
            .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
            .draw(self)?;
        self.fill_solid(
            &Rectangle::new(Point::new(113, 88), Size::new(20, 7)),
            BinaryColor::Off,
        )?;

        Text::new("Total", Point::new(114, 93), desc_text_style).draw(self)?;
        SUN_PATTERN
            .iter()
            .enumerate()
            .map(|(idx, num)| {
                let x = 102 + (idx % 18);
                let y = 95 + (idx / 18);
                let color = {
                    if num == &0 {
                        BinaryColor::Off
                    } else if num == &1 {
                        BinaryColor::On
                    } else {
                        BinaryColor::Off
                    }
                };
                Pixel(Point::new(x as i32, y as i32), color)
            })
            .draw(self)?;

        if solar.len() > 3 {
            Text::new(solar, Point::new(122, 104), desc_text_style).draw(self)?;
        } else {
            Text::new(solar, Point::new(125, 104), desc_text_style).draw(self)?;
        }
        HOUSE_PATTERN
            .iter()
            .enumerate()
            .map(|(idx, num)| {
                let x = 102 + (idx % 18);
                let y = 112 + (idx / 18);
                let color = {
                    if num == &0 {
                        BinaryColor::Off
                    } else if num == &1 {
                        BinaryColor::On
                    } else {
                        BinaryColor::Off
                    }
                };
                Pixel(Point::new(x as i32, y as i32), color)
            })
            .draw(self)?;

        if house.len() > 3 {
            Text::new(house, Point::new(122, 121), desc_text_style).draw(self)?;
        } else {
            Text::new(house, Point::new(125, 121), desc_text_style).draw(self)?;
        }

        Ok(())
    }

    pub fn update_total_new(&mut self, house: &str, solar: &str) -> anyhow::Result<()> {
        self.fill_solid(
            &Rectangle::new(Point::new(120, 97), Size::new(23, 30)),
            BinaryColor::Off,
        )?;
        let desc_text_style = MonoTextStyleBuilder::new()
            .font(&embedded_graphics::mono_font::ascii::FONT_4X6)
            .text_color(BinaryColor::On)
            .build();
        if solar.len() > 3 {
            Text::new(solar, Point::new(122, 104), desc_text_style).draw(self)?;
        } else {
            Text::new(solar, Point::new(125, 104), desc_text_style).draw(self)?;
        }
        if house.len() > 3 {
            Text::new(house, Point::new(122, 121), desc_text_style).draw(self)?;
        } else {
            Text::new(house, Point::new(125, 121), desc_text_style).draw(self)?;
        }
        Ok(())
    }
    pub fn draw_default_display<'a>(
        &mut self,
        style: MonoTextStyle<'a, BinaryColor>,
    ) -> anyhow::Result<()> {
        self.draw_default_battery_percentage()?;
        //Circle top
        Circle::new(Point::new(55, 2), 40)
            .into_styled(
                PrimitiveStyleBuilder::new()
                    .stroke_color(BinaryColor::On)
                    .stroke_width(2)
                    .build(),
            )
            .draw(self)?;
        // Circle buttom
        Circle::new(Point::new(55, 86), 40)
            .into_styled(
                PrimitiveStyleBuilder::new()
                    .stroke_color(BinaryColor::On)
                    .stroke_width(2)
                    .build(),
            )
            .draw(self)?;

        // Circle left
        Circle::new(Point::new(13, 44), 40)
            .into_styled(
                PrimitiveStyleBuilder::new()
                    .stroke_width(2)
                    .stroke_color(BinaryColor::On)
                    .build(),
            )
            .draw(self)?;

        //Circle right
        Circle::new(Point::new(97, 44), 40)
            .into_styled(
                PrimitiveStyleBuilder::new()
                    .stroke_width(2)
                    .stroke_color(BinaryColor::On)
                    .build(),
            )
            .draw(self)?;
        // Solids for the icons
        self.fill_solid(
            &Rectangle::new(Point::new(66, 0), Size::new(18, 15)),
            BinaryColor::Off,
        )?;
        self.fill_solid(
            &Rectangle::new(Point::new(66, 84), Size::new(18, 15)),
            BinaryColor::Off,
        )?;
        self.fill_solid(
            &Rectangle::new(Point::new(24, 42), Size::new(18, 15)),
            BinaryColor::Off,
        )?;
        self.fill_solid(
            &Rectangle::new(Point::new(108, 42), Size::new(18, 15)),
            BinaryColor::Off,
        )?;

        // icons
        HOUSE_PATTERN
            .iter()
            .enumerate()
            .map(|(idx, num)| {
                let x = 66 + (idx % 18);
                let y = 0 + (idx / 18);
                let color = {
                    if num == &0 {
                        BinaryColor::Off
                    } else if num == &1 {
                        BinaryColor::On
                    } else {
                        BinaryColor::Off
                    }
                };
                Pixel(Point::new(x as i32, y as i32), color)
            })
            .draw(self)?;
        LIGHTNING_BOLT_PATTERN
            .iter()
            .enumerate()
            .map(|(idx, num)| {
                let x = 108 + (idx % 18);
                let y = 43 + (idx / 18);
                let color = {
                    if num == &0 {
                        BinaryColor::Off
                    } else if num == &1 {
                        BinaryColor::On
                    } else {
                        BinaryColor::Off
                    }
                };
                Pixel(Point::new(x as i32, y as i32), color)
            })
            .draw(self)?;
        BATTERY_PATTERN
            .iter()
            .enumerate()
            .map(|(idx, num)| {
                let x = 66 + (idx % 18);
                let y = 84 + (idx / 18);
                let color = {
                    if num == &0 {
                        BinaryColor::Off
                    } else if num == &1 {
                        BinaryColor::On
                    } else {
                        BinaryColor::Off
                    }
                };
                Pixel(Point::new(x as i32, y as i32), color)
            })
            .draw(self)?;
        Line::new(Point::new(66 + 12, 85), Point::new(71, 96))
            .into_styled(
                PrimitiveStyleBuilder::new()
                    .stroke_color(BinaryColor::On)
                    .stroke_width(1)
                    .build(),
            )
            .draw(self)?;

        SUN_PATTERN
            .iter()
            .enumerate()
            .map(|(idx, num)| {
                let x = 24 + (idx % 18);
                let y = 42 + (idx / 18);
                let color = {
                    if num == &0 {
                        BinaryColor::Off
                    } else if num == &1 {
                        BinaryColor::On
                    } else {
                        BinaryColor::Off
                    }
                };
                Pixel(Point::new(x as i32, y as i32), color)
            })
            .draw(self)?;
        Line::new(Point::new(149, 0), Point::new(149, 128))
            .into_styled(
                PrimitiveStyleBuilder::new()
                    .stroke_color(BinaryColor::On)
                    .stroke_width(2)
                    .build(),
            )
            .draw(self)?;
        Line::new(Point::new(103, 0), Point::new(103, 20))
            .into_styled(
                PrimitiveStyleBuilder::new()
                    .stroke_color(BinaryColor::On)
                    .stroke_width(2)
                    .build(),
            )
            .draw(self)?;
        Line::new(Point::new(103, 20), Point::new(149, 20))
            .into_styled(
                PrimitiveStyleBuilder::new()
                    .stroke_color(BinaryColor::On)
                    .stroke_width(2)
                    .build(),
            )
            .draw(self)?;

        Text::new(
            "%",
            Point::new(71, 120),
            MonoTextStyleBuilder::new()
                .font(&embedded_graphics::mono_font::ascii::FONT_9X15)
                .text_color(BinaryColor::On)
                .build(),
        )
        .draw(self)?;

        let pos = [(65, 23), (22, 65), (107, 65)];
        pos.iter().try_for_each(|pos| -> anyhow::Result<()> {
            Text::new(
                "kW",
                Point::new(pos.0 + 5, pos.1 + 11),
                MonoTextStyleBuilder::new()
                    .font(&embedded_graphics::mono_font::ascii::FONT_6X10)
                    .text_color(BinaryColor::On)
                    .build(),
            )
            .draw(self)?;
            Ok(())
        })?;

        Text::new(
            "%",
            Point::new(71, 120),
            MonoTextStyleBuilder::new()
                .font(&embedded_graphics::mono_font::ascii::FONT_9X15)
                .text_color(BinaryColor::On)
                .build(),
        )
        .draw(self)?;
        self.draw_text(style, "0.00", "-0.00", "0.00", "-0.00", "0:00PM")?;
        self.draw_default_weather()?;
        self.new_total("00.00", "00.00")?;
        Ok(())
    }
    pub fn draw_text<'a>(
        &mut self,
        style: MonoTextStyle<'a, BinaryColor>,
        circle_top: &'a str,
        circle_bottom: &'a str,
        circle_left: &'a str,
        circle_right: &'a str,
        update: &'a str,
    ) -> anyhow::Result<()> {
        // Circle top
        // normally we have 3 digits
        if circle_top.len() == 1 {
            Text::new(circle_top, Point::new(74, 23), style).draw(self)?;
        } else {
            Text::new(circle_top, Point::new(65, 23), style).draw(self)?;
        }

        // Circle bottom
        if circle_bottom.len() == 3 {
            Text::new(circle_bottom, Point::new(68, 107), style).draw(self)?;
        } else if circle_bottom.len() == 4 {
            Text::new(circle_bottom, Point::new(64, 107), style).draw(self)?;
        } else {
            Text::new(circle_bottom, Point::new(60, 107), style).draw(self)?;
        }

        // Circle left
        if circle_left.len() == 1 {
            Text::new(circle_left, Point::new(31, 65), style).draw(self)?;
        } else {
            Text::new(circle_left, Point::new(22, 65), style).draw(self)?;
        }

        // Circle right
        if circle_right.len() == 2 {
            Text::new(circle_right, Point::new(111, 65), style).draw(self)?;
        } else {
            Text::new(circle_right, Point::new(103, 65), style).draw(self)?;
        }

        // Time
        if update.len() == 6 {
            Text::new(update, Point::new(110, 10), style).draw(self)?;
        } else {
            Text::new(update, Point::new(107, 10), style).draw(self)?;
        }

        Ok(())
    }
    pub fn clear_text(&mut self) -> anyhow::Result<()> {
        self.fill_solid(
            &Rectangle::new(
                Point::new(65, 15),
                embedded_graphics::prelude::Size::new(25, 10),
            ),
            BinaryColor::Off,
        )?;
        self.fill_solid(
            &Rectangle::new(
                Point::new(60, 99),
                embedded_graphics::prelude::Size::new(30, 10),
            ),
            BinaryColor::Off,
        )?;
        self.fill_solid(
            &Rectangle::new(
                Point::new(22, 57),
                embedded_graphics::prelude::Size::new(25, 10),
            ),
            BinaryColor::Off,
        )?;
        self.fill_solid(
            &Rectangle::new(
                Point::new(102, 57),
                embedded_graphics::prelude::Size::new(30, 10),
            ),
            BinaryColor::Off,
        )?;
        self.fill_solid(
            &Rectangle::new(
                Point::new(105, 1),
                embedded_graphics::prelude::Size::new(42, 18),
            ),
            BinaryColor::Off,
        )?;
        return Ok(());
    }
    pub fn display_error_message<'a>(
        &mut self,
        message: &str,
        style: MonoTextStyle<'a, BinaryColor>,
    ) -> anyhow::Result<()> {
        Text::new(message, Point::new(58, 100), style).draw(self)?;
        Ok(())
    }
    pub fn draw_connections(&mut self, connection: ConnectionDirection) -> anyhow::Result<()> {
        match connection {
            ConnectionDirection::Top(arr) => {
                // Line middle to top circle
                Line::new(Point::new(75, 64), Point::new(75, 44))
                    .into_styled(
                        PrimitiveStyleBuilder::new()
                            .stroke_color(BinaryColor::On)
                            .stroke_width(2)
                            .build(),
                    )
                    .draw(self)?;
                match arr {
                    true => {
                        self.draw_arrow(ArrowDirection::Up)?;
                    }
                    false => (),
                }
            }
            ConnectionDirection::Left(arr) => {
                //Line middle to left circle
                Line::new(Point::new(74, 63), Point::new(55, 63))
                    .into_styled(
                        PrimitiveStyleBuilder::new()
                            .stroke_color(BinaryColor::On)
                            .stroke_width(2)
                            .build(),
                    )
                    .draw(self)?;
                match arr {
                    true => {
                        self.draw_arrow(ArrowDirection::Left)?;
                    }
                    false => (),
                }
            }
            ConnectionDirection::Right(arr) => {
                // Line middle to right circle
                Line::new(Point::new(74, 64), Point::new(94, 64))
                    .into_styled(
                        PrimitiveStyleBuilder::new()
                            .stroke_color(BinaryColor::On)
                            .stroke_width(2)
                            .build(),
                    )
                    .draw(self)?;
                match arr {
                    true => {
                        self.draw_arrow(ArrowDirection::Right)?;
                    }
                    false => (),
                }
            }
            ConnectionDirection::Bottom(arr) => {
                // Line middle to bottom circle
                Line::new(Point::new(74, 64), Point::new(74, 82))
                    .into_styled(
                        PrimitiveStyleBuilder::new()
                            .stroke_color(BinaryColor::On)
                            .stroke_width(2)
                            .build(),
                    )
                    .draw(self)?;

                match arr {
                    true => {
                        self.draw_arrow(ArrowDirection::Down)?;
                    }
                    false => (),
                }
            }
        }

        Ok(())
    }
    pub fn set_connected(&mut self) -> anyhow::Result<()> {
        Line::new(Point::new(0, 118), Point::new(40, 118))
            .into_styled(
                PrimitiveStyleBuilder::new()
                    .stroke_width(1)
                    .stroke_color(BinaryColor::On)
                    .build(),
            )
            .draw(self)?;
        Line::new(Point::new(40, 118), Point::new(40, 128))
            .into_styled(
                PrimitiveStyleBuilder::new()
                    .stroke_width(1)
                    .stroke_color(BinaryColor::On)
                    .build(),
            )
            .draw(self)?;
        Text::new(
            "Connected",
            Point::new(2, 125),
            MonoTextStyleBuilder::new()
                .font(&embedded_graphics::mono_font::ascii::FONT_4X6)
                .text_color(BinaryColor::On)
                .build(),
        )
        .draw(self)?;
        Ok(())
    }
    fn draw_arrow(&mut self, direction: ArrowDirection) -> anyhow::Result<()> {
        match direction {
            ArrowDirection::Up => {
                // Arrow up
                Line::new(Point::new(75, 44), Point::new(82, 51))
                    .into_styled(
                        PrimitiveStyleBuilder::new()
                            .stroke_width(1)
                            .stroke_color(BinaryColor::On)
                            .build(),
                    )
                    .draw(self)?;
                Line::new(Point::new(74, 44), Point::new(67, 51))
                    .into_styled(
                        PrimitiveStyleBuilder::new()
                            .stroke_width(1)
                            .stroke_color(BinaryColor::On)
                            .build(),
                    )
                    .draw(self)?;
                // Arrow up end
            }
            ArrowDirection::Right => {
                // Arrow right
                Line::new(Point::new(94, 63), Point::new(87, 56))
                    .into_styled(
                        PrimitiveStyleBuilder::new()
                            .stroke_width(1)
                            .stroke_color(BinaryColor::On)
                            .build(),
                    )
                    .draw(self)?;
                Line::new(Point::new(94, 64), Point::new(87, 71))
                    .into_styled(
                        PrimitiveStyleBuilder::new()
                            .stroke_width(1)
                            .stroke_color(BinaryColor::On)
                            .build(),
                    )
                    .draw(self)?;
                //arrow right end
            }
            ArrowDirection::Down => {
                // Arrow down
                Line::new(Point::new(74, 82), Point::new(67, 75))
                    .into_styled(
                        PrimitiveStyleBuilder::new()
                            .stroke_width(1)
                            .stroke_color(BinaryColor::On)
                            .build(),
                    )
                    .draw(self)?;
                Line::new(Point::new(75, 82), Point::new(82, 75))
                    .into_styled(
                        PrimitiveStyleBuilder::new()
                            .stroke_width(1)
                            .stroke_color(BinaryColor::On)
                            .build(),
                    )
                    .draw(self)?;
                //arrow down end
            }
            ArrowDirection::Left => {
                // Arrow left
                Line::new(Point::new(55, 64), Point::new(62, 71))
                    .into_styled(
                        PrimitiveStyleBuilder::new()
                            .stroke_width(1)
                            .stroke_color(BinaryColor::On)
                            .build(),
                    )
                    .draw(self)?;
                Line::new(Point::new(55, 63), Point::new(62, 56))
                    .into_styled(
                        PrimitiveStyleBuilder::new()
                            .stroke_width(1)
                            .stroke_color(BinaryColor::On)
                            .build(),
                    )
                    .draw(self)?;
                //arrow left end
            }
        }
        Ok(())
    }
    pub fn update_battery_percentage(&mut self, percentage: &str) -> anyhow::Result<()> {
        self.fill_solid(
            &Rectangle::new(Point::new(1, 1), Size::new(28, 12)),
            BinaryColor::Off,
        )?;
        if percentage.len() > 3 || percentage.len() < 1 {
            return Err(anyhow!("errro input sequence too long"));
        }
        let offset = {
            if percentage.len() == 3 {
                0
            } else if percentage.len() == 2 {
                5
            } else {
                10
            }
        };
        Text::new(
            &format!("{percentage}%"),
            Point::new(3 + offset, 10),
            MonoTextStyleBuilder::new()
                .font(&embedded_graphics::mono_font::ascii::FONT_6X10)
                .text_color(BinaryColor::On)
                .build(),
        )
        .draw(self)?;

        Ok(())
    }
    fn draw_default_battery_percentage(&mut self) -> anyhow::Result<()> {
        Line::new(Point::new(30, 0), Point::new(30, 15))
            .into_styled(
                PrimitiveStyleBuilder::new()
                    .stroke_color(BinaryColor::On)
                    .stroke_width(2)
                    .build(),
            )
            .draw(self)?;

        Line::new(Point::new(0, 15), Point::new(30, 15))
            .into_styled(
                PrimitiveStyleBuilder::new()
                    .stroke_color(BinaryColor::On)
                    .stroke_width(2)
                    .build(),
            )
            .draw(self)?;
        Text::new(
            "100%",
            Point::new(3, 10),
            MonoTextStyleBuilder::new()
                .font(&embedded_graphics::mono_font::ascii::FONT_6X10)
                .text_color(BinaryColor::On)
                .build(),
        )
        .draw(self)?;
        Ok(())
    }
    pub fn update_sun_data(&mut self, sunrise: &str, sunset: &str) -> anyhow::Result<()> {
        let style = MonoTextStyleBuilder::new()
            .font(&embedded_graphics::mono_font::ascii::FONT_6X10)
            .text_color(BinaryColor::On)
            .build();
        if sunset.len() > 5 || sunrise.len() > 5 {
            return Err(anyhow!("error input sequence to long"));
        }

        // clears both text areas
        self.fill_solid(
            &Rectangle::new(Point::new(190, 3), Size::new(30, 12)),
            BinaryColor::Off,
        )?;
        self.fill_solid(
            &Rectangle::new(Point::new(260, 3), Size::new(30, 12)),
            BinaryColor::Off,
        )?;

        Text::new(sunrise, Point::new(190, 12), style).draw(self)?;

        Text::new(sunset, Point::new(260, 12), style).draw(self)?;
        Ok(())
    }

    pub fn draw_default_weather(&mut self) -> anyhow::Result<()> {
        // first split display on the right into 2;

        Line::new(Point::new(149, 70), Point::new(296, 70))
            .into_styled(
                PrimitiveStyleBuilder::new()
                    .stroke_color(BinaryColor::On)
                    .stroke_width(2)
                    .build(),
            )
            .draw(self)?;
        SUN_PATTERN
            .iter()
            .enumerate()
            .map(|(idx, num)| {
                let x = 155 + (idx % 18);
                let y = 2 + (idx / 18);
                let color = {
                    if num == &0 {
                        BinaryColor::Off
                    } else if num == &1 {
                        BinaryColor::On
                    } else {
                        BinaryColor::Off
                    }
                };
                Pixel(Point::new(x as i32, y as i32), color)
            })
            .draw(self)?;

        // arrow up
        self.draw_arrow_simple(SimpleArrowDirection::Up, (178, 3))?;

        SUN_PATTERN
            .iter()
            .enumerate()
            .map(|(idx, num)| {
                let x = 225 + (idx % 18);
                let y = 2 + (idx / 18);
                let color = {
                    if num == &0 {
                        BinaryColor::Off
                    } else if num == &1 {
                        BinaryColor::On
                    } else {
                        BinaryColor::Off
                    }
                };
                Pixel(Point::new(x as i32, y as i32), color)
            })
            .draw(self)?;
        // arrow down
        self.draw_arrow_simple(SimpleArrowDirection::Down, (248, 3))?;
        let style = MonoTextStyleBuilder::new()
            .font(&embedded_graphics::mono_font::ascii::FONT_6X10)
            .text_color(BinaryColor::On)
            .build();

        // sunset and sunrise values
        Text::new("00.00", Point::new(190, 12), style).draw(self)?;
        Text::new("00.00", Point::new(260, 12), style).draw(self)?;

        // other descriptors
        RAINDROPS
            .iter()
            .enumerate()
            .map(|(idx, num)| {
                let x = 155 + (idx % 18);
                let y = 15 + (idx / 18);
                let color = {
                    if num == &0 {
                        BinaryColor::Off
                    } else if num == &1 {
                        BinaryColor::On
                    } else {
                        BinaryColor::Off
                    }
                };
                Pixel(Point::new(x as i32, y as i32), color)
            })
            .draw(self)?;
        CLOUD
            .iter()
            .enumerate()
            .map(|(idx, num)| {
                let x = 155 + (idx % 18);
                let y = 28 + (idx / 18);
                let color = {
                    if num == &0 {
                        BinaryColor::Off
                    } else if num == &1 {
                        BinaryColor::On
                    } else {
                        BinaryColor::Off
                    }
                };
                Pixel(Point::new(x as i32, y as i32), color)
            })
            .draw(self)?;

        Text::new("UV", Point::new(160, 50), style).draw(self)?;
        TEMP.iter()
            .enumerate()
            .map(|(idx, num)| {
                let x = 156 + (idx % 18);
                let y = 52 + (idx / 18);
                let color = {
                    if num == &0 {
                        BinaryColor::Off
                    } else if num == &1 {
                        BinaryColor::On
                    } else {
                        BinaryColor::Off
                    }
                };
                Pixel(Point::new(x as i32, y as i32), color)
            })
            .draw(self)?;
        let offsets = &[20, 50, 80, 110];
        for x_offset in offsets.iter() {
            self.draw_row_weather_data("0.0", "100.0", "0.0", "10.0", x_offset.clone())?;
        }
        Line::new(Point::new(203, 18), Point::new(203, 65))
            .into_styled(
                PrimitiveStyleBuilder::new()
                    .stroke_width(1)
                    .stroke_color(BinaryColor::On)
                    .build(),
            )
            .draw(self)?;

        Line::new(Point::new(232, 18), Point::new(232, 65))
            .into_styled(
                PrimitiveStyleBuilder::new()
                    .stroke_width(1)
                    .stroke_color(BinaryColor::On)
                    .build(),
            )
            .draw(self)?;

        Line::new(Point::new(261, 18), Point::new(261, 65))
            .into_styled(
                PrimitiveStyleBuilder::new()
                    .stroke_width(1)
                    .stroke_color(BinaryColor::On)
                    .build(),
            )
            .draw(self)?;

        Ok(())
    }
    fn draw_row_weather_data<'a>(
        &mut self,
        rain: &'a str,
        cloud: &'a str,
        uv: &'a str,
        temp: &'a str,
        x_offset: i32,
    ) -> anyhow::Result<()> {
        let style_2 = MonoTextStyleBuilder::new()
            .font(&embedded_graphics::mono_font::ascii::FONT_5X8)
            .text_color(BinaryColor::On)
            .build();
        if rain.len() == 3 {
            Text::new(rain, Point::new(155 + x_offset + 10, 27), style_2).draw(self)?;
        } else if rain.len() == 4 {
            Text::new(rain, Point::new(155 + x_offset + 5, 27), style_2).draw(self)?;
        } else {
            Text::new(rain, Point::new(155 + x_offset, 27), style_2).draw(self)?;
        }
        if cloud.len() == 3 {
            Text::new(cloud, Point::new(155 + x_offset + 10, 38), style_2).draw(self)?;
        } else if cloud.len() == 4 {
            Text::new(cloud, Point::new(155 + x_offset + 5, 38), style_2).draw(self)?;
        } else {
            Text::new(cloud, Point::new(155 + x_offset, 38), style_2).draw(self)?;
        }
        if uv.len() == 3 {
            Text::new(uv, Point::new(155 + x_offset + 10, 50), style_2).draw(self)?;
        } else if uv.len() == 4 {
            Text::new(uv, Point::new(155 + x_offset + 5, 50), style_2).draw(self)?;
        } else {
            Text::new(uv, Point::new(155 + x_offset, 50), style_2).draw(self)?;
        }
        if temp.len() == 3 {
            Text::new(temp, Point::new(155 + x_offset + 10, 62), style_2).draw(self)?;
        } else if temp.len() == 4 {
            Text::new(temp, Point::new(155 + x_offset + 5, 62), style_2).draw(self)?;
        } else {
            Text::new(temp, Point::new(155 + x_offset, 62), style_2).draw(self)?;
        }

        Ok(())
    }
    pub fn update_weather_data(&mut self, weather_data: HourlyNew) -> anyhow::Result<()> {
        let offsets = &[20, 50, 80, 110];
        self.fill_solid(
            &Rectangle::new(Point::new(172, 18), Size::new(130, 50)),
            BinaryColor::Off,
        )?;
        for (idx, x_offset) in offsets.iter().enumerate() {
            let rain = weather_data
                .rain
                .get(idx)
                .ok_or(anyhow!("error missing data"))?;
            let cloud = weather_data
                .cloud_cover
                .get(idx)
                .ok_or(anyhow!("error missing data"))?;
            let uv = weather_data
                .uv_index
                .get(idx)
                .ok_or(anyhow!("error missing data"))?;
            let temp = weather_data
                .temperature_2m
                .get(idx)
                .ok_or(anyhow!("error missing data"))?;

            self.draw_row_weather_data(rain, cloud, uv, temp, x_offset.clone())?
        }

        // seperation lines
        Line::new(Point::new(203, 18), Point::new(203, 65))
            .into_styled(
                PrimitiveStyleBuilder::new()
                    .stroke_width(1)
                    .stroke_color(BinaryColor::On)
                    .build(),
            )
            .draw(self)?;

        Line::new(Point::new(232, 18), Point::new(232, 65))
            .into_styled(
                PrimitiveStyleBuilder::new()
                    .stroke_width(1)
                    .stroke_color(BinaryColor::On)
                    .build(),
            )
            .draw(self)?;
        Line::new(Point::new(261, 18), Point::new(261, 65))
            .into_styled(
                PrimitiveStyleBuilder::new()
                    .stroke_width(1)
                    .stroke_color(BinaryColor::On)
                    .build(),
            )
            .draw(self)?;

        Ok(())
    }
    fn draw_arrow_simple(
        &mut self,
        direction: SimpleArrowDirection,
        startpos: (i32, i32),
    ) -> anyhow::Result<()> {
        match direction {
            SimpleArrowDirection::Up => {
                Line::new(
                    Point::new(startpos.0, startpos.1),
                    Point::new(startpos.0, startpos.1 + 11),
                )
                .into_styled(
                    PrimitiveStyleBuilder::new()
                        .stroke_width(2)
                        .stroke_color(BinaryColor::On)
                        .build(),
                )
                .draw(self)?;
                Line::new(
                    Point::new(startpos.0, startpos.1),
                    Point::new(startpos.0 + 6, startpos.1 + 6),
                )
                .into_styled(
                    PrimitiveStyleBuilder::new()
                        .stroke_width(2)
                        .stroke_color(BinaryColor::On)
                        .build(),
                )
                .draw(self)?;
                Line::new(
                    Point::new(startpos.0, startpos.1),
                    Point::new(startpos.0 - 6, startpos.1 + 6),
                )
                .into_styled(
                    PrimitiveStyleBuilder::new()
                        .stroke_width(2)
                        .stroke_color(BinaryColor::On)
                        .build(),
                )
                .draw(self)?;
                Pixel(Point::new(startpos.0 - 6, startpos.1 + 6), BinaryColor::Off).draw(self)?;
            }
            SimpleArrowDirection::Down => {
                Line::new(
                    Point::new(startpos.0, startpos.1),
                    Point::new(startpos.0, startpos.1 + 11),
                )
                .into_styled(
                    PrimitiveStyleBuilder::new()
                        .stroke_width(2)
                        .stroke_color(BinaryColor::On)
                        .build(),
                )
                .draw(self)?;
                Line::new(
                    Point::new(startpos.0 + 1, startpos.1 + 11),
                    Point::new(startpos.0 - 5, startpos.1 + 11 - 6),
                )
                .into_styled(
                    PrimitiveStyleBuilder::new()
                        .stroke_width(2)
                        .stroke_color(BinaryColor::On)
                        .build(),
                )
                .draw(self)?;
                Line::new(
                    Point::new(startpos.0 + 1, startpos.1 + 11),
                    Point::new(startpos.0 + 7, startpos.1 + 11 - 6),
                )
                .into_styled(
                    PrimitiveStyleBuilder::new()
                        .stroke_width(2)
                        .stroke_color(BinaryColor::On)
                        .build(),
                )
                .draw(self)?;
                Pixel(
                    Point::new(startpos.0 + 7, startpos.1 + 11 - 6),
                    BinaryColor::Off,
                )
                .draw(self)?;
            }
        }

        Ok(())
    }
    pub fn draw_status_message(&mut self, msg: &str) -> anyhow::Result<()> {
        let style = MonoTextStyleBuilder::new()
            .font(&embedded_graphics::mono_font::ascii::FONT_9X15)
            .text_color(BinaryColor::On)
            .build();
        Text::new(msg, Point::new(58, 64), style).draw(self)?;
        Ok(())
    }
    pub fn clear_status_message(&mut self) -> anyhow::Result<()> {
        self.fill_solid(
            &Rectangle::new(
                Point::new(58, 50),
                embedded_graphics::prelude::Size::new(180, 20),
            ),
            BinaryColor::Off,
        )?;
        Ok(())
    }
    fn draw_default_total(&mut self) -> anyhow::Result<()> {
        let style = MonoTextStyleBuilder::new()
            .font(&embedded_graphics::mono_font::ascii::FONT_9X15)
            .text_color(BinaryColor::On)
            .build();
        let style_total = MonoTextStyleBuilder::new()
            .font(&embedded_graphics::mono_font::ascii::FONT_6X12)
            .text_color(BinaryColor::On)
            .build();

        // top right corner display "Total"
        Text::new("Total", Point::new(154, 80), style_total).draw(self)?;
        Line::new(Point::new(150, 86), Point::new(190, 86))
            .into_styled(
                PrimitiveStyleBuilder::new()
                    .stroke_width(2)
                    .stroke_color(BinaryColor::On)
                    .build(),
            )
            .draw(self)?;
        Line::new(Point::new(190, 86), Point::new(190, 70))
            .into_styled(
                PrimitiveStyleBuilder::new()
                    .stroke_width(2)
                    .stroke_color(BinaryColor::On)
                    .build(),
            )
            .draw(self)?;
        // end
        //
        SUN_PATTERN
            .iter()
            .enumerate()
            .map(|(idx, num)| {
                let x = 180 + (idx % 18);
                let y = 90 + (idx / 18);
                let color = {
                    if num == &0 {
                        BinaryColor::Off
                    } else if num == &1 {
                        BinaryColor::On
                    } else {
                        BinaryColor::Off
                    }
                };
                Pixel(Point::new(x as i32, y as i32), color)
            })
            .draw(self)?;
        Text::new("00.00 kW", Point::new(205, 100), style).draw(self)?;

        HOUSE_PATTERN
            .iter()
            .enumerate()
            .map(|(idx, num)| {
                let x = 180 + (idx % 18);
                let y = 110 + (idx / 18);
                let color = {
                    if num == &0 {
                        BinaryColor::Off
                    } else if num == &1 {
                        BinaryColor::On
                    } else {
                        BinaryColor::Off
                    }
                };
                Pixel(Point::new(x as i32, y as i32), color)
            })
            .draw(self)?;

        Text::new("00.00 kW", Point::new(205, 120), style).draw(self)?;
        Ok(())
    }
    pub fn update_total_display(
        &mut self,
        consumption: &str,
        generated: &str,
    ) -> anyhow::Result<()> {
        if consumption.len() > 5 || generated.len() > 5 {
            return Err(anyhow!("input to long"));
        }
        let style = MonoTextStyleBuilder::new()
            .font(&embedded_graphics::mono_font::ascii::FONT_9X15)
            .text_color(BinaryColor::On)
            .build();
        self.fill_solid(
            &Rectangle::new(Point::new(205, 90), Size::new(45, 12)),
            BinaryColor::Off,
        )?;
        self.fill_solid(
            &Rectangle::new(Point::new(205, 110), Size::new(45, 12)),
            BinaryColor::Off,
        )?;

        Text::new(generated, Point::new(205, 100), style).draw(self)?;
        Text::new(consumption, Point::new(205, 120), style).draw(self)?;
        Ok(())
    }
    pub fn update_chart(&mut self, data: &[f32]) -> anyhow::Result<()> {
        self.fill_solid(
            &Rectangle::new(Point::new(142, 76), Size::new(30, 5)),
            BinaryColor::Off,
        )?;
        self.fill_solid(
            &Rectangle::new(Point::new(151, 75), Size::new(146, 47)),
            BinaryColor::Off,
        )?;
        self.draw_chart(data)?;

        Ok(())
    }
}
