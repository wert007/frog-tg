use image::{DynamicImage, Pixel, Rgba};
use rusttype::{Font, Scale};
use text_on_image::FontBundle;

const TEMPLATE: &'static [u8] = include_bytes!("../../assets/template.png");
const FONT: &'static [u8] = include_bytes!("../../assets/fonts/Coolvetica Rg.otf");

pub(crate) fn create_image_report(walk: &crate::CompleteWalk) -> anyhow::Result<Vec<u8>> {
    let mut img = image::load_from_memory(TEMPLATE)?;

    write(&mut img, walk.start.format("%d.%m.%Y"), 196, 243);
    write(&mut img, walk.start.format("%H:%m"), 220, 317);
    if let Some(end) = walk.end {
        write(&mut img, end.format("%H:%m"), 220, 351);
    }
    write_weather(&mut img, walk.weather);
    img.save("output.png")?;
    todo!()
}

fn write_weather(img: &mut DynamicImage, weather: crate::weather::WeatherStats) {
    write(
        img,
        if let Some(end) = weather.temperature_end
            && (weather.temperature_start - end).abs() > 3.0
        {
            format!("{:.1}°C - {end:.1}°C", weather.temperature_start)
        } else {
            format!("{:.1}°C", weather.temperature_start)
        },
        600,
        243,
    );
    write(img, weather.wind_beaufort, 930, 243);
    write(
        img,
        format!(
            "{}/{}/{}",
            weather.ground_humidity, weather.percipation, weather.cloudiness
        ),
        1500,
        243,
    );
}

fn write(img: &mut DynamicImage, text: impl ToString, x: i32, y: i32) {
    let font = Font::try_from_bytes(FONT).unwrap();
    let font_bundle = FontBundle::new(
        &font,
        Scale::uniform(30.0),
        *Rgba::from_slice(&[0u8, 0, 0, 0xff]),
    );
    text_on_image::text_on_image(
        img,
        text.to_string(),
        &font_bundle,
        x,
        y,
        text_on_image::TextJustify::Left,
        text_on_image::VerticalAnchor::Top,
        text_on_image::WrapBehavior::NoWrap,
    );
}
