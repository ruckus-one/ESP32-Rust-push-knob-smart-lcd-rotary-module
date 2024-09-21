use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::Result;

use embedded_graphics::mono_font::ascii::{FONT_10X20};
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::prelude::Size;
use embedded_graphics::primitives::Rectangle;
use embedded_graphics::text::{Alignment, LineHeight, Text, TextStyleBuilder};
use esp_idf_hal::delay::{Delay, FreeRtos};
use esp_idf_hal::gpio::{self, OutputPin, PinDriver};
use esp_idf_hal::spi::{
    self,
    config::{Config, Mode, Phase, Polarity},
    SpiDeviceDriver,
};

use esp_idf_svc::
    hal::peripherals::Peripherals
;

use embedded_graphics::{
    pixelcolor::Rgb565,
    prelude::{Point, RgbColor},
    primitives::{Primitive, PrimitiveStyleBuilder},
    Drawable,
};
use gc9a01::{mode::BufferedGraphics, prelude::*, Gc9a01, SPIDisplayInterface};

mod rotencoder;
use rotencoder::Rotencoder;

mod push_button;
use push_button::{Button, ButtonState};

fn draw<I: WriteOnlyDataCommand, D: DisplayDefinition>(
    display: &mut Gc9a01<I, D, BufferedGraphics<D>>,
    counter: i32,
    is_pressed: bool,
) {

    // begin - clear
    let style = PrimitiveStyleBuilder::new()
        .fill_color(Rgb565::BLACK)
        .build();

    Rectangle::new(Point::new(0, 105), Size::new(240, 20))
        .into_styled(style)
        .draw(display)
        .unwrap();
    
    Rectangle::new(Point::new(40, 125), Size::new(200, 20))
    .into_styled(style)
    .draw(display)
    .unwrap();
    // end - clear


    // Create a new character style.
    let character_style = MonoTextStyle::new(&FONT_10X20, Rgb565::WHITE);

    // Create a new text style.
    let text_style = TextStyleBuilder::new()
        .alignment(Alignment::Center)
        .line_height(LineHeight::Percent(1000))
        .build();

    let text = format!("Counter -> {}", counter);
    Text::with_text_style(
        text.as_str(),
        Point::new(120, 120),
        character_style,
        text_style,
    )
    .draw(display)
    .unwrap();
    

    if is_pressed {
        let style = MonoTextStyle::new(&FONT_10X20, Rgb565::RED);
        let text = format!("Button -> Pressed");
        Text::new(text.as_str(), Point::new(60, 140), style)
            .draw(display)
            .unwrap();
    }

}

type BoxedDisplayDriver<'a> = Box<
    Gc9a01<
        SPIInterface<
            SpiDeviceDriver<'a, spi::SpiDriver<'a>>,
            PinDriver<'a, gpio::AnyOutputPin, gpio::Output>,
        >,
        DisplayResolution240x240,
        BufferedGraphics<DisplayResolution240x240>,
    >,
>;

fn main() -> Result<()> {
    esp_idf_svc::sys::link_patches();

    let peripherals = Peripherals::take().unwrap();
    let pins = peripherals.pins;
    let mut delay = Delay::new_default();    

    let sck = pins.gpio1;
    let mosi = pins.gpio0;
    let cs = pins.gpio10;
    let dc = pins.gpio4;
    let reset = pins.gpio2;
    let backlight = pins.gpio8;

    let enc_a = pins.gpio6;
    let enc_b = pins.gpio7;

    let btn = pins.gpio9;

    let cs_output = cs;
    let dc_output = PinDriver::output(dc.downgrade_output()).unwrap();
    let mut backlight_output = PinDriver::output(backlight.downgrade_output()).unwrap();
    let mut reset_output = PinDriver::output(reset.downgrade_output()).unwrap();

    backlight_output.set_low().unwrap();

    let driver = spi::SpiDriver::new(
        peripherals.spi2,
        sck,
        mosi,
        None::<gpio::AnyIOPin>,
        &spi::SpiDriverConfig::new(),
    )
    .unwrap();

    let config = Config::new().baudrate(2_000_000.into()).data_mode(Mode {
        polarity: Polarity::IdleLow,
        phase: Phase::CaptureOnFirstTransition,
    });

    let spi_device = SpiDeviceDriver::new(driver, Some(cs_output), &config).unwrap();

    let interface = SPIDisplayInterface::new(spi_device, dc_output);

    let mut display_driver: BoxedDisplayDriver = Box::new(
        Gc9a01::new(
            interface,
            DisplayResolution240x240,
            DisplayRotation::Rotate0,
        )
        .into_buffered_graphics(),
    );

    display_driver.reset(&mut reset_output, &mut delay).ok();
    display_driver.init(&mut delay).ok();
    log::info!("Driver configured!");

    let counter = Arc::new(AtomicI32::new(0));
    let _rotencoder_handle = {
        let counter = counter.clone();

        let encoder = Rotencoder::with_callback(
            enc_a,
            enc_b,
            Arc::new(Mutex::new(move |delta: i8| {

                match delta {
                    1 => counter.fetch_add(1, Ordering::SeqCst),
                    -1 => counter.fetch_sub(1, Ordering::SeqCst),
                    _ => 0_i32,
                };
            }))
        );

        encoder.start_thread()
    };

    let is_pressed = Arc::new(AtomicBool::new(false));
    let _btn_handle = {
        let is_pressed = is_pressed.clone();

        let btn = Button::new(btn, Arc::new(Mutex::new(move |state: ButtonState| {
            match state {
                ButtonState::Pressed => {
                    // println!("Pressed");
                    is_pressed.store(true, Ordering::SeqCst);
                },
                ButtonState::Released => {
                    // println!("Released");
                    is_pressed.store(false, Ordering::SeqCst);
                },
            }
        })));

        btn.spawn_thread()
    };

    loop {
        // display_driver.clear();
        draw(&mut display_driver, counter.load(Ordering::SeqCst), is_pressed.load(Ordering::SeqCst));
        display_driver.flush().ok();
        FreeRtos::delay_ms(10);
    }
}