use unicode_normalization::{char::is_combining_mark, UnicodeNormalization};

pub fn normalize(s: &str) -> String {
    s.nfd()
        .filter(|c| !is_combining_mark(*c))
        .collect::<String>()
        .to_lowercase()
}
