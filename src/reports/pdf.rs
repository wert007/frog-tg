use std::{borrow::Cow, collections::HashMap};

use chrono::Timelike;
use lopdf::{
    Document, FontData, Object, Stream,
    content::{Content, Operation},
    dictionary,
};

use crate::{CompleteWalk, FrogFound, weather::WeatherStats};

const FONT: &[u8] = include_bytes!("../../assets/fonts/Coolvetica Rg.otf");
const TEMPLATE: &[u8] = include_bytes!("../../assets/template.pdf");

#[derive(Debug, Default)]
struct FrogCountSpeciesLocation {
    male: usize,
    female: usize,
    unknown: usize,
}
impl FrogCountSpeciesLocation {
    fn update(&mut self, frog: &FrogFound) {
        match frog.sex {
            crate::Sex::Male => self.male += 1,
            crate::Sex::Female => self.female += 1,
            crate::Sex::Unknown => self.unknown += 1,
        }
    }
}

#[derive(Debug, Default)]
struct FrogCountSpecies {
    towards: [FrogCountSpeciesLocation; 2],
    backwards: [FrogCountSpeciesLocation; 2],
}
impl FrogCountSpecies {
    fn update(&mut self, frog: &FrogFound) {
        if frog.towards {
            self.towards[frog.location].update(frog);
        } else {
            self.backwards[frog.location].update(frog);
        }
    }
}

#[derive(Debug, Default)]
struct FrogCount {
    species: HashMap<String, FrogCountSpecies>,
}

impl FrogCount {
    fn new(frogs: &[FrogFound]) -> Self {
        let mut result = Self::default();
        for frog in frogs {
            result
                .species
                .entry(frog.name.clone())
                .or_default()
                .update(frog);
        }
        result
    }

    fn fill_in(&self, doc: &mut Document, page_id: (u32, u16)) -> anyhow::Result<()> {
        for (species, count) in &self.species {
            let position = position_from_species(&species);
            for i in 0..2 {
                write(
                    doc,
                    to_text(species, count.towards[i].male),
                    12,
                    add(position, OFFSET_MALE, OFFSET_TOWARDS, OFFSET_LOCATION[i]),
                    page_id,
                )?;
                write(
                    doc,
                    to_text(species, count.towards[i].female),
                    12,
                    add(position, OFFSET_FEMALE, OFFSET_TOWARDS, OFFSET_LOCATION[i]),
                    page_id,
                )?;
                write(
                    doc,
                    to_text(species, count.towards[i].unknown),
                    12,
                    add(position, OFFSET_UNKNOWN, OFFSET_TOWARDS, OFFSET_LOCATION[i]),
                    page_id,
                )?;
                write(
                    doc,
                    to_text(species, count.backwards[i].male),
                    12,
                    add(position, OFFSET_MALE, OFFSET_BACKWARDS, OFFSET_LOCATION[i]),
                    page_id,
                )?;
                write(
                    doc,
                    to_text(species, count.backwards[i].female),
                    12,
                    add(
                        position,
                        OFFSET_FEMALE,
                        OFFSET_BACKWARDS,
                        OFFSET_LOCATION[i],
                    ),
                    page_id,
                )?;
                write(
                    doc,
                    to_text(species, count.backwards[i].unknown),
                    12,
                    add(
                        position,
                        OFFSET_UNKNOWN,
                        OFFSET_BACKWARDS,
                        OFFSET_LOCATION[i],
                    ),
                    page_id,
                )?;
            }
        }
        Ok(())
    }
}

fn to_text(species: &str, count: usize) -> String {
    if [
        "Erdkröte",
        "Grasfrosch",
        "Teichmolch",
        "Bergmolch",
        "Kammmolch",
    ]
    .contains(&species)
    {
        if count > 0 {
            count.to_string()
        } else {
            "/".into()
        }
    } else {
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
                unknown => format!("{count} {unknown}"),
            }
        } else {
            String::new()
        }
    }
}

const OFFSET_MALE: [i32; 2] = [0, 0];
const OFFSET_FEMALE: [i32; 2] = [70, 0];
const OFFSET_UNKNOWN: [i32; 2] = [140, 0];
const OFFSET_TOWARDS: [i32; 2] = [0, 0];
const OFFSET_BACKWARDS: [i32; 2] = [0, 35];
const OFFSET_LOCATION: [[i32; 2]; 2] = [[0, 0], [0, 70]];

fn add(
    position: [i32; 2],
    offset_male: [i32; 2],
    offset_towards: [i32; 2],
    offset_location: [i32; 2],
) -> [i32; 2] {
    [
        position[0] + offset_male[0] + offset_towards[0] + offset_location[0],
        position[1] + offset_male[1] + offset_towards[1] + offset_location[1],
    ]
}

fn position_from_species(species: &str) -> [i32; 2] {
    match species {
        "Erdkröte" => [225, 210],
        "Grasfrosch" => [435, 210],
        "Teichmolch" => [225, 490],
        "Bergmolch" => [435, 490],
        "Kammmolch" => [645, 490],
        _ => [625, 210],
    }
}

pub fn create_pdf_report(walk: &CompleteWalk) -> anyhow::Result<()> {
    let mut doc = lopdf::Document::load_mem(TEMPLATE)?;
    doc.add_font(FontData::new(FONT, "default".into()))?;

    let pages = doc.get_pages();
    let page_id = *pages.get(&1).unwrap();

    write_date(&mut doc, page_id, walk.start)?;
    write_weather(&mut doc, page_id, walk.weather)?;
    write_time(&mut doc, page_id, walk.start, walk.end)?;

    let frog_count = FrogCount::new(&walk.frogs);
    frog_count.fill_in(&mut doc, page_id)?;

    doc.save("output.pdf")?;

    Ok(())
}

fn write_time(
    doc: &mut Document,
    page_id: (u32, u16),
    start: chrono::DateTime<chrono::Local>,
    end: Option<chrono::DateTime<chrono::Local>>,
) -> anyhow::Result<()> {
    write(
        doc,
        start.format("%H:%M").to_string(),
        12,
        [105, 163],
        page_id,
    )?;
    if let Some(end) = end {
        write(
            doc,
            end.format("%H:%M").to_string(),
            12,
            [105, 178],
            page_id,
        )?;
    }
    if start.hour() <= 12 {
        draw_line(doc, page_id, [58, 365], [58, 405], 2)?;
    } else {
        draw_line(doc, page_id, [58, 420], [58, 445], 2)?;
    }
    Ok(())
}

fn write_weather(
    doc: &mut Document,
    page_id: (u32, u16),
    weather: WeatherStats,
) -> anyhow::Result<()> {
    let temp = if let Some(end) = weather.temperature_end {
        if (end - weather.temperature_start) > 3.0 {
            format!("{}°C-{}°C", weather.temperature_start, end)
        } else {
            format!("{}°C", weather.temperature_start)
        }
    } else {
        format!("{}°C", weather.temperature_start)
    };
    write(doc, temp, 12, [290, 129], page_id)?;
    write(
        doc,
        weather.wind_beaufort.to_string(),
        12,
        [460, 129],
        page_id,
    )?;
    let text = format!(
        "{}/{}/{}",
        match weather.ground_humidity {
            crate::weather::GroundHumidity::Unknown => "?",
            crate::weather::GroundHumidity::Wet => "nass",
            crate::weather::GroundHumidity::Humid => "feucht",
            crate::weather::GroundHumidity::Dry => "trocken",
            crate::weather::GroundHumidity::VeryDry => "sehr trocken",
        },
        match weather.percipation {
            crate::weather::Percipation::None => "keiner",
            crate::weather::Percipation::StrongRain => "Starker Regen",
            crate::weather::Percipation::ModerateRain => "Mäßiger Regen",
            crate::weather::Percipation::Drizzle => "Niesel",
            crate::weather::Percipation::Fog => "Nebel",
            crate::weather::Percipation::Snow => "Schnee",
            crate::weather::Percipation::Graupel => "Graupel",
            crate::weather::Percipation::Unknown => "?",
        },
        match weather.cloudiness {
            crate::weather::Cloudiness::AllClouds => "bedeckt".into(),
            crate::weather::Cloudiness::ManyClouds => "stark bewölkt".into(),
            crate::weather::Cloudiness::Clouds => "bewölkt".into(),
            crate::weather::Cloudiness::FewClouds => "leicht bewölkt".into(),
            crate::weather::Cloudiness::Clear => "wolkenlos".into(),
            crate::weather::Cloudiness::GettingCloudy => "zuziehend".into(),
            crate::weather::Cloudiness::GettingClear => "aufklarend".into(),
            crate::weather::Cloudiness::Error(clouds) => Cow::Owned(format!("? (clouds: {clouds}")),
        }
    );
    write(doc, text, 12, [720, 129], page_id)?;
    Ok(())
}

fn write_date(
    doc: &mut Document,
    page_id: (u32, u16),
    start: chrono::DateTime<chrono::Local>,
) -> anyhow::Result<()> {
    write(
        doc,
        start.format("%d.%m.%Y").to_string(),
        12,
        [100, 129],
        page_id,
    )
}

fn write(
    doc: &mut Document,
    text: impl AsRef<str>,
    size: i32,
    position: [i32; 2],
    page_id: (u32, u16),
) -> anyhow::Result<()> {
    let content = Content {
        operations: vec![
            Operation::new("BT", vec![]),
            // font + size
            Operation::new("Tf", vec!["default".into(), size.into()]),
            Operation::new(
                "Tm",
                vec![
                    1.into(),
                    0.into(),
                    0.into(),
                    (-1).into(),
                    position[0].into(),
                    position[1].into(),
                ],
            ),
            Operation::new(
                "Tj",
                vec![Object::string_literal(
                    encoding_rs::WINDOWS_1252.encode(text.as_ref()).0.to_vec(),
                )],
            ),
            Operation::new("ET", vec![]),
        ],
    };

    let stream = Stream::new(dictionary! {}, content.encode()?);
    let content_id = doc.add_object(stream);

    {
        let page = doc.get_object_mut(page_id)?.as_dict_mut()?;

        match page.get_mut(b"Contents") {
            Ok(Object::Reference(existing)) => {
                let existing = *existing;
                page.set(
                    "Contents",
                    Object::Array(vec![
                        Object::Reference(existing),
                        Object::Reference(content_id),
                    ]),
                );
            }
            Ok(Object::Array(arr)) => {
                arr.push(Object::Reference(content_id));
            }
            _ => {
                page.set("Contents", content_id);
            }
        }
    }
    Ok(())
}

fn draw_line(
    doc: &mut Document,
    page_id: (u32, u16),
    from: [i32; 2],
    to: [i32; 2],
    width: i32,
) -> anyhow::Result<()> {
    let content = Content {
        operations: vec![
            Operation::new("q", vec![]), // save graphics state
            Operation::new("w", vec![width.into()]),
            Operation::new("m", vec![from[0].into(), from[1].into()]), // move to
            Operation::new("l", vec![to[0].into(), to[1].into()]),     // line to
            Operation::new("S", vec![]),                               // stroke line
            Operation::new("Q", vec![]),                               // restore graphics state
        ],
    };

    let stream = Stream::new(dictionary! {}, content.encode()?);
    let content_id = doc.add_object(stream);

    let page = doc.get_object_mut(page_id)?.as_dict_mut()?;

    match page.get_mut(b"Contents") {
        Ok(Object::Reference(existing)) => {
            let existing = *existing;
            page.set(
                "Contents",
                Object::Array(vec![
                    Object::Reference(existing),
                    Object::Reference(content_id),
                ]),
            );
        }
        Ok(Object::Array(arr)) => arr.push(Object::Reference(content_id)),
        _ => page.set("Contents", content_id),
    }

    Ok(())
}
