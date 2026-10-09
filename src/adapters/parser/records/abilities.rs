//! Ability blocks: CA, PA, team, position ratings and attributes, after the people.
//!
//! ```text
//! [person id - 1 u32][uid u32][source u32] .. [ca][00][pa][00] .. [team u32] ..
//! [15 position ratings][54 attributes]
//! ```
//! A block is read from its attributes, the part that is easiest to recognise.
use super::{
    super::bytes::u32,
    people::Person,
    scan::{is_set, scan},
};
use crate::{utils::cancel::CancelFlag, Result};

pub struct Ability {
    /// Where the attributes start.
    pub offset: usize,
    pub person_id: u64,
    pub ca: u8,
    pub pa: u8,
    pub team: u32,
    pub positions: Vec<u8>,
    pub raw: Vec<u8>,
}
impl Ability {
    /// Bytes from the person id to the attributes.
    const BEFORE: usize = 57;
    pub(super) const ATTRIBUTES: usize = 54;

    /// The block whose attributes start at `offset`.
    fn read(db: &[u8], offset: usize) -> Option<Self> {
        let (ca, pa) = (db[offset - 38], db[offset - 36]);
        let positions = &db[offset - 15..offset];
        let attributes = &db[offset..offset + Self::ATTRIBUTES];
        let ok = db[offset - 37] == 0
            && db[offset - 35] == 0
            && (1..=200).contains(&ca)
            && (1..=200).contains(&pa)
            && positions.iter().all(|v| (1..=20).contains(v))
            && positions.contains(&20)
            && attributes.iter().all(|v| (1..=100).contains(v))
            && is_set(u32(db, offset - 53))
            && is_set(u32(db, offset - 49));
        ok.then(|| Self {
            offset,
            person_id: u32(db, offset - 57) as u64 + 1,
            ca,
            pa,
            team: u32(db, offset - 23),
            positions: positions.to_vec(),
            raw: attributes.to_vec(),
        })
    }
}

/// Every ability block after `start` (the end of the name pools).
pub fn find(db: &[u8], start: usize, cancel: &CancelFlag) -> Result<Vec<Ability>> {
    let to = db.len().saturating_sub(Ability::ATTRIBUTES);
    scan(start + Ability::BEFORE, to, cancel, |offset| {
        Ability::read(db, offset).map(|block| (block, offset + Ability::ATTRIBUTES))
    })
}

/// People paired with their ability block, and how many could not be paired.
pub struct Linked<'a> {
    pub pairs: Vec<(&'a Person, &'a Ability)>,
    /// People with more than one ability block before them.
    pub ambiguous: usize,
    /// People whose one ability block belongs to someone else.
    pub unbound: usize,
}
impl Linked<'_> {
    pub fn warnings(&self) -> Vec<String> {
        let excluded = self.ambiguous + self.unbound;
        if excluded == 0 {
            return vec![];
        }
        vec![format!(
            "{excluded} ambiguous player records were excluded rather than assigned uncertain ratings."
        )]
    }
}

/// Pairs each person with the single ability block between the previous person and them,
/// if that block names them as its owner.
pub fn link<'a>(people: &'a [Person], abilities: &'a [Ability]) -> Linked<'a> {
    let mut linked = Linked {
        pairs: vec![],
        ambiguous: 0,
        unbound: 0,
    };
    let mut ix = 0;
    for person in people {
        let first = ix;
        while ix < abilities.len() && abilities[ix].offset < person.offset {
            ix += 1;
        }
        match ix - first {
            0 => {}
            1 if person.id >= 0 && abilities[first].person_id == person.id as u64 => {
                linked.pairs.push((person, &abilities[first]))
            }
            1 => linked.unbound += 1,
            _ => linked.ambiguous += 1,
        }
    }
    linked
}
