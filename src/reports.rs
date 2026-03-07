use std::collections::HashMap;

use crate::FrogFound;

mod pdf;

pub fn create_pdf_report(walk: &crate::CompleteWalk) -> anyhow::Result<()> {
    pdf::create_pdf_report(walk)
}

pub(crate) fn create_inline_end_walk_report(walk: &crate::CompleteWalk) -> String {
    use std::fmt::Write;
    if walk.frogs.is_empty() && walk.dead_frogs.is_empty() {
        return String::new();
    }
    let mut result = String::new();
    _ = writeln!(result, "");
    _ = writeln!(result, "");
    for (name, frog) in frogs_by_names(&walk.frogs) {
        let emoji = match name.as_str() {
            "Erdkröte" | "Knoblauchkröte" | "Grasfrosch" | "Laubfrosch" | "Springfrosch"
            | "Grünfrosch" => "🐸",
            "Bergmolch" | "Teichmolch" | "Kammmolch" => "🦎",
            _ => "",
        };
        _ = writeln!(result, " - You found {} {name} {emoji}!", frog.len());
    }
    result
}

fn frogs_by_names(frogs: &[FrogFound]) -> FrogByNames<'_> {
    let mut map: HashMap<String, Vec<&FrogFound>> = HashMap::new();
    for frog in frogs {
        map.entry(frog.name.clone()).or_default().push(frog);
    }
    FrogByNames { map }
}

struct FrogByNames<'a> {
    map: HashMap<String, Vec<&'a FrogFound>>,
}

impl<'a> IntoIterator for FrogByNames<'a> {
    type Item = <HashMap<String, Vec<&'a FrogFound>> as IntoIterator>::Item;

    type IntoIter = <HashMap<String, Vec<&'a FrogFound>> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.map.into_iter()
    }
}
