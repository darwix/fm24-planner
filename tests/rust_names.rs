use fm_savelens_backend::parser::records::display_name;

#[test]
fn nickname_wins_over_first_and_last() {
    assert_eq!(display_name("Ronaldo", "Silva", Some("Ronald")), "Ronald");
}

#[test]
fn no_nickname_falls_back_to_first_and_last() {
    assert_eq!(display_name("Ricardo", "Forniés", None), "Ricardo Forniés");
    assert_eq!(display_name("Ricardo", "Forniés", Some("")), "Ricardo Forniés");
    assert_eq!(display_name("", "Pelé", None), "Pelé");
}
