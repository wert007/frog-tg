use std::io::{BufWriter, Cursor};

use chrono::Timelike;
use image::{DynamicImage, ImageFormat, Pixel, Rgba};
use imageproc::pixelops::interpolate;
use rusttype::{Font, Scale};
use text_on_image::FontBundle;

use crate::counting::{DeadFrogCount, FrogCount};

const TEMPLATE: &'static [u8] = include_bytes!("../../assets/template.png");
const FONT: &'static [u8] = include_bytes!("../../assets/fonts/Coolvetica Rg.otf");

trait ImageRenderable {
    fn fill_in(&self, img: &mut DynamicImage);
}

pub(crate) fn create_image_report(walk: &crate::CompleteWalk) -> anyhow::Result<Vec<u8>> {
    let mut img = image::load_from_memory(TEMPLATE)?;

    write_time(&mut img, walk);
    write_weather(&mut img, walk.weather);
    FrogCount::new(&walk.frogs).fill_in(&mut img);
    DeadFrogCount::new(&walk.dead_frogs).fill_in(&mut img);

    write(&mut img, include_str!("../../author.txt"), 215, 1489);

    let mut w = BufWriter::new(Cursor::new(Vec::new()));
    img.write_to(&mut w, ImageFormat::Png)?;
    Ok(w.into_inner()?.into_inner())
}

fn write_time(img: &mut DynamicImage, walk: &crate::CompleteWalk) {
    write(img, walk.start.format("%d.%m.%Y"), 196, 243);
    write(img, walk.start.format("%H:%m"), 220, 317);
    if let Some(end) = walk.end {
        write(img, end.format("%H:%m"), 220, 351);
    }
    for y in (110..126).step_by(3) {
        let (start, end) = if walk.start.hour() > 12 {
            ((y, 870), (y, 930))
        } else {
            ((y, 760), (y, 840))
        };
        imageproc::drawing::draw_antialiased_line_segment_mut(
            img,
            start,
            end,
            *Rgba::from_slice(&[0u8, 0, 0, 0xff]),
            interpolate,
        );
    }
}

fn display_named(species: &str, count: usize) -> String {
    if count > 0 {
        match species {
            "Kröte" => format!("{count} Kr"),
            "Frosch" => format!("{count} Fr"),
            "Molch" => format!("{count} M"),
            "Erdkröte" => format!("{count} EKr"),
            "Knoblauchkröte" => format!("{count} KnKr"),
            "Grasfrosch" => format!("{count} GFr"),
            "Springfrosch" => format!("{count} SFr"),
            "Grünfrosch" => format!("{count} GrFr"),
            "Laubfrosch" => format!("{count} LFr"),
            "Teichmolch" => format!("{count} TM"),
            "Bergmolch" => format!("{count} BM"),
            "Kammmolch" => format!("{count} KM"),
            "Feuersalamander" => format!("{count} FS"),
            "" => format!("{count} ?"),
            unknown => format!("{count} {unknown}"),
        }
    } else {
        String::new()
    }
}

fn display(number: usize) -> String {
    if number > 0 {
        number.to_string()
    } else {
        String::new()
    }
}

impl ImageRenderable for DeadFrogCount {
    fn fill_in(&self, img: &mut DynamicImage) {
        let mut text = String::new();
        for (species, count) in &self.found {
            text.push_str(&display_named(species, *count));
            text.push('\n');
        }
        write_centered(img, text, 1700, 700);
    }
}

impl ImageRenderable for FrogCount {
    fn fill_in(&self, img: &mut DynamicImage) {
        const LOCATION_HEIGHT: i32 = 162;
        const SEX_WIDTH: i32 = 146;
        for (species, count) in self.known_species() {
            let (x, y) = position_from_species(&species);
            let total_frog_count = count.total();
            write_centered(
                img,
                display(total_frog_count.total()),
                x + SEX_WIDTH * 3 / 2,
                if species.ends_with("molch") {
                    1415
                } else {
                    830
                },
            );
            for i in 0..3 {
                write_centered(
                    img,
                    display(count.towards(i).male),
                    x + SEX_WIDTH / 2,
                    y + i as i32 * LOCATION_HEIGHT,
                );
                write_centered(
                    img,
                    display(count.towards(i).female),
                    x + SEX_WIDTH / 2 + SEX_WIDTH,
                    y + i as i32 * LOCATION_HEIGHT,
                );
                write_centered(
                    img,
                    display(count.towards(i).unknown),
                    x + SEX_WIDTH / 2 + 2 * SEX_WIDTH,
                    y + i as i32 * LOCATION_HEIGHT,
                );
                write_centered(
                    img,
                    display(count.backwards(i).male),
                    x + SEX_WIDTH / 2,
                    y + i as i32 * LOCATION_HEIGHT + LOCATION_HEIGHT / 2,
                );
                write_centered(
                    img,
                    display(count.backwards(i).female),
                    x + SEX_WIDTH / 2 + SEX_WIDTH,
                    y + i as i32 * LOCATION_HEIGHT + LOCATION_HEIGHT / 2,
                );
                write_centered(
                    img,
                    display(count.backwards(i).unknown),
                    x + SEX_WIDTH / 2 + 2 * SEX_WIDTH,
                    y + i as i32 * LOCATION_HEIGHT + LOCATION_HEIGHT / 2,
                );
            }
        }
        let r = self.remaining();
        let (x, y) = position_from_species("");
        for i in 0..3 {
            write_centered(
                img,
                r.towards(i).format_male(|s, c| display_named(s, c)),
                x + SEX_WIDTH / 2,
                y + i as i32 * LOCATION_HEIGHT,
            );
            write_centered(
                img,
                r.towards(i).format_female(|s, c| display_named(s, c)),
                x + SEX_WIDTH / 2 + SEX_WIDTH,
                y + i as i32 * LOCATION_HEIGHT,
            );
            write_centered(
                img,
                r.towards(i).format_unknown(|s, c| display_named(s, c)),
                x + SEX_WIDTH / 2 + 2 * SEX_WIDTH,
                y + i as i32 * LOCATION_HEIGHT,
            );
            write_centered(
                img,
                r.backwards(i).format_male(|s, c| display_named(s, c)),
                x + SEX_WIDTH / 2,
                y + LOCATION_HEIGHT / 2 + i as i32 * LOCATION_HEIGHT,
            );
            write_centered(
                img,
                r.backwards(i).format_female(|s, c| display_named(s, c)),
                x + SEX_WIDTH / 2 + SEX_WIDTH,
                y + LOCATION_HEIGHT / 2 + i as i32 * LOCATION_HEIGHT,
            );
            write_centered(
                img,
                r.backwards(i).format_unknown(|s, c| display_named(s, c)),
                x + SEX_WIDTH / 2 + 2 * SEX_WIDTH,
                y + LOCATION_HEIGHT / 2 + i as i32 * LOCATION_HEIGHT,
            );
        }
    }
}

fn position_from_species(species: &str) -> (i32, i32) {
    match species {
        "Erdkröte" => (406, 387),
        "Grasfrosch" => (840, 387),
        "Teichmolch" => (406, 969),
        "Bergmolch" => (840, 969),
        "Kammmolch" => (1277, 969),
        _ => (1277, 387),
    }
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

fn write_centered(img: &mut DynamicImage, text: impl ToString, x: i32, y: i32) {
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
        text_on_image::TextJustify::Center,
        text_on_image::VerticalAnchor::Top,
        text_on_image::WrapBehavior::NoWrap,
    );
}
