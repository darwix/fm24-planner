use fm_squad_planner::adapters::parser::records::people::display_name;

#[test]
fn nickname_wins_over_first_and_last() {
    assert_eq!(display_name("Ronaldo", "Silva", Some("Ronald")), "Ronald");
}

#[test]
fn no_nickname_falls_back_to_first_and_last() {
    assert_eq!(display_name("Ricardo", "Forniés", None), "Ricardo Forniés");
    assert_eq!(
        display_name("Ricardo", "Forniés", Some("")),
        "Ricardo Forniés"
    );
    assert_eq!(display_name("", "Pelé", None), "Pelé");
}

// The nickname index reads from three pools (checked on Francisco Miguel, Juanma, Casemiro).
#[test]
fn nickname_index_picks_common_then_own_first_then_last_pool() {
    use fm_squad_planner::adapters::parser::records::{names::Names, people::nickname};
    let pool = |v: &[&str]| v.iter().map(|s| s.to_string()).collect();
    let names = Names {
        first: pool(&["Juan", "Francisco Miguel", "Pedro"]),
        last: pool(&["Delgado", "Gámez", "López", "Juanma"]),
        common: pool(&["Casemiro"]),
        start: 0,
        end: 0,
    };
    assert_eq!(nickname(&names, 1, 0), Some("Casemiro"));
    assert_eq!(nickname(&names, 1, 1), Some("Francisco Miguel"));
    assert_eq!(nickname(&names, 0, 3), Some("Juanma"));
    assert_eq!(nickname(&names, 0, u32::MAX), None);
}
