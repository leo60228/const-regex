use const_regex::match_regex;

const fn is_star_wars<const N: usize>(subtitle: &[u8; N]) -> bool {
    let mut bytes = *subtitle;

    let mut i = 0;
    while i < bytes.len() {
        bytes[i] = bytes[i].to_ascii_lowercase();
        i += 1;
    }

    match_regex!("m | [tn]|b", &bytes)
}

fn main() {
    dbg!(is_star_wars(b"The Phantom Menace"));
    dbg!(is_star_wars(b"Attack of the Clones"));
    dbg!(is_star_wars(b"The Empire Strikes Back"));
}
