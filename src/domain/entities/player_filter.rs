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
    Position,
    Value,
    Wage,
    ContractEnd,
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
    /// ISO date; players with no contract match only with `or_no_contract`.
    pub contract_end_max: Option<String>,
    pub or_no_contract: bool,
    /// Only players on the snapshot's shortlist.
    pub shortlist: bool,
    /// Leave out this club's players (free agents always stay).
    pub exclude_club: Option<u32>,
    pub no_contract: bool,
    pub transfer_listed: bool,
    pub loan_listed: bool,
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
            or_no_contract: false,
            shortlist: false,
            exclude_club: None,
            no_contract: false,
            transfer_listed: false,
            loan_listed: false,
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
