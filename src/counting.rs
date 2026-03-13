use std::collections::HashMap;

use crate::FrogFound;

pub struct DeadFrogCount {
    found: HashMap<String, usize>,
}
impl DeadFrogCount {
    pub fn new(dead_frogs: &[crate::DeadFrog]) -> Self {
        let mut result = Self {
            found: Default::default(),
        };
        for frog in dead_frogs {
            let name = frog.name.clone().unwrap_or_default();
            *result.found.entry(name).or_default() += 1;
        }
        result
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct FrogCountSpeciesLocation {
    pub male: usize,
    pub female: usize,
    pub unknown: usize,
}
impl FrogCountSpeciesLocation {
    fn update(&mut self, frog: &FrogFound) {
        match frog.sex {
            crate::Sex::Male => self.male += 1,
            crate::Sex::Female => self.female += 1,
            crate::Sex::Unknown => self.unknown += 1,
        }
    }

    pub fn total(&self) -> usize {
        self.male + self.female + self.unknown
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct FrogCountSpecies {
    towards: [FrogCountSpeciesLocation; 3],
    backwards: [FrogCountSpeciesLocation; 3],
}
impl FrogCountSpecies {
    fn update(&mut self, frog: &FrogFound) {
        if frog.towards {
            self.towards[frog.location].update(frog);
        } else {
            self.backwards[frog.location].update(frog);
        }
    }

    pub fn total(&self) -> FrogCountSpeciesLocation {
        FrogCountSpeciesLocation {
            male: self.total_male(),
            female: self.total_female(),
            unknown: self.total_unknown(),
        }
    }

    pub fn towards(&self, i: usize) -> FrogCountSpeciesLocation {
        self.towards[i]
    }

    pub fn backwards(&self, i: usize) -> FrogCountSpeciesLocation {
        self.backwards[i]
    }

    fn total_male(&self) -> usize {
        self.towards.map(|t| t.male).iter().sum::<usize>()
            + self.backwards.map(|b| b.male).iter().sum::<usize>()
    }

    fn total_female(&self) -> usize {
        self.towards.map(|t| t.female).iter().sum::<usize>()
            + self.backwards.map(|b| b.female).iter().sum::<usize>()
    }

    fn total_unknown(&self) -> usize {
        self.towards.map(|t| t.unknown).iter().sum::<usize>()
            + self.backwards.map(|b| b.unknown).iter().sum::<usize>()
    }
}

#[derive(Debug, Default)]
pub struct FrogCount {
    species: HashMap<String, FrogCountSpecies>,
}

impl FrogCount {
    pub fn new(frogs: &[FrogFound]) -> Self {
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

    pub(crate) fn known_species(&self) -> [(&'static str, FrogCountSpecies); 5] {
        [
            "Erdkröte",
            "Grasfrosch",
            "Teichmolch",
            "Bergmolch",
            "Kammmolch",
        ]
        .map(|s| (s, self.species.get(s).copied().unwrap_or_default()))
    }
    // fn fill_in(&self, doc: &mut Document, page_id: (u32, u16)) -> anyhow::Result<()> {
    //     for (species, count) in &self.species {
    //         let position = position_from_species(&species);
    //         let total_frog_count = count.total();
    //         for i in 0..2 {
    //             write(
    //                 doc,
    //                 to_text(species, count.towards[i].male),
    //                 12,
    //                 add(position, OFFSET_MALE, OFFSET_TOWARDS, OFFSET_LOCATION[i]),
    //                 page_id,
    //             )?;
    //             write(
    //                 doc,
    //                 to_text(species, count.towards[i].female),
    //                 12,
    //                 add(position, OFFSET_FEMALE, OFFSET_TOWARDS, OFFSET_LOCATION[i]),
    //                 page_id,
    //             )?;
    //             write(
    //                 doc,
    //                 to_text(species, count.towards[i].unknown),
    //                 12,
    //                 add(position, OFFSET_UNKNOWN, OFFSET_TOWARDS, OFFSET_LOCATION[i]),
    //                 page_id,
    //             )?;
    //             write(
    //                 doc,
    //                 to_text(species, count.backwards[i].male),
    //                 12,
    //                 add(position, OFFSET_MALE, OFFSET_BACKWARDS, OFFSET_LOCATION[i]),
    //                 page_id,
    //             )?;
    //             write(
    //                 doc,
    //                 to_text(species, count.backwards[i].female),
    //                 12,
    //                 add(
    //                     position,
    //                     OFFSET_FEMALE,
    //                     OFFSET_BACKWARDS,
    //                     OFFSET_LOCATION[i],
    //                 ),
    //                 page_id,
    //             )?;
    //             write(
    //                 doc,
    //                 to_text(species, count.backwards[i].unknown),
    //                 12,
    //                 add(
    //                     position,
    //                     OFFSET_UNKNOWN,
    //                     OFFSET_BACKWARDS,
    //                     OFFSET_LOCATION[i],
    //                 ),
    //                 page_id,
    //             )?;
    //         }
    //         // write(
    //         //     doc,
    //         //     sum_to_text(total_frog_count.male),
    //         //     12,
    //         //     add(position, OFFSET_MALE, OFFSET_SUM, [0; 2]),
    //         //     page_id,
    //         // )?;
    //         // write(
    //         //     doc,
    //         //     sum_to_text(total_frog_count.female),
    //         //     12,
    //         //     add(position, OFFSET_FEMALE, OFFSET_SUM, [0; 2]),
    //         //     page_id,
    //         // )?;
    //         // write(
    //         //     doc,
    //         //     sum_to_text(total_frog_count.unknown),
    //         //     12,
    //         //     add(position, OFFSET_UNKNOWN, OFFSET_SUM, [0; 2]),
    //         //     page_id,
    //         // )?;
    //         write(
    //             doc,
    //             sum_to_text(total_frog_count.total()),
    //             12,
    //             add(position, [0; 2], OFFSET_SUM, [0; 2]),
    //             page_id,
    //         )?;
    //     }
    //     Ok(())
    // }
}
