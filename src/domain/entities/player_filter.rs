/// An optional lower and upper bound, both inclusive.
#[derive(Clone, Copy, Default)]
pub struct Range {
    pub min: Option<i64>,
    pub max: Option<i64>,
}

pub enum ClubFilter {
    /// Players with no club.
    Free,
    Id(u32),
}

/// Positions a player must be able to play: all of them or any of them.
pub struct PositionFilter {
    pub mask: i64,
    pub all: bool,
}

pub enum SortKey {
    Name,
    Club,
    Age,
    Ca,
    Pa,
    Attribute(String),
}

/// What a player search asks for, already validated.
pub struct PlayerFilter {
    /// Normalised text to find in the player's names.
    pub text: Option<String>,
    pub age: Range,
    pub ca: Range,
    pub pa: Range,
    pub value_max: Option<i64>,
    /// With `value_max`: also keep players whose value is unknown.
    pub value_unknown: bool,
    pub main_positions: Vec<usize>,
    pub exclude_ids: Vec<u32>,
    /// ISO date; players with no contract never match.
    pub contract_end_max: Option<String>,
    pub no_contract: bool,
    pub club: Option<ClubFilter>,
    pub nation: Option<i64>,
    pub positions: Option<PositionFilter>,
    /// Attribute key and the minimum rating.
    pub attributes: Vec<(String, i64)>,
    pub sort: SortKey,
    pub descending: bool,
    pub page: i64,
    pub limit: i64,
}
impl Default for PlayerFilter {
    fn default() -> Self {
        Self {
            text: None,
            age: Range::default(),
            ca: Range::default(),
            pa: Range::default(),
            value_max: None,
            value_unknown: false,
            main_positions: vec![],
            exclude_ids: vec![],
            contract_end_max: None,
            no_contract: false,
            club: None,
            nation: None,
            positions: None,
            attributes: vec![],
            sort: SortKey::Pa,
            descending: true,
            page: 1,
            limit: 100,
        }
    }
}
