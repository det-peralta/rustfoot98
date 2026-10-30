//! Number and money formatting of the original (Portuguese, space as the thousands separator).

/// `1216448` -> `"1 216 448"`, negative numbers keep their sign.
pub fn int(v: i64) -> String {
    let s = v.abs().to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(' ');
        }
        out.push(ch);
    }
    if v < 0 {
        format!("-{out}")
    } else {
        out
    }
}

/// An amount in words as the game writes it (`1040:335a`): `1 milhão e 216 mil reais`, `975 mil reais`, `mil reais`, `um milhão de reais`,
/// `20 milhões de reais`, `2 milhões e 500 mil reais`, `850 reais`.
pub fn money_words(v: i64, unit: &str) -> String {
    let v = v.max(0);
    let (m, k, r) = (v / 1_000_000, v % 1_000_000 / 1000, v % 1000);
    if v == 1000 {
        return format!("mil {unit}");
    }
    if v == 1_000_000 {
        return format!("um milhão de {unit}");
    }
    if m == 0 && k == 0 {
        return format!("{r} {unit}");
    }
    if m >= 2 && k == 0 && r == 0 {
        return format!("{m} milhões de {unit}");
    }
    let mut parts = Vec::new();
    if m > 0 {
        parts.push(format!("{m} {}", if m == 1 { "milhão" } else { "milhões" }));
    }
    if k > 0 {
        parts.push(format!("{k} mil"));
    }
    let body = parts.join(" e ");
    format!("{body} {unit}")
}

/// The behaviour word of a player by their aggression (`0..=5`): the table of `10c8:171a`, one word per value.
pub fn behaviour(aggression: u8) -> &'static str {
    const WORDS: [&str; 6] = [
        "Fair play",
        "Cordeirinho",
        "Cavalheiro",
        "Caneleiro",
        "Caceteiro",
        "Sarrafeiro",
    ];
    WORDS[(aggression as usize).min(5)]
}

pub const POSITIONS: [&str; 4] = ["Guarda-redes", "Defesa", "Médio", "Avançado"];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_and_words() {
        assert_eq!(int(1216448), "1 216 448");
        assert_eq!(int(-81000), "-81 000");
        assert_eq!(int(0), "0");
        assert_eq!(money_words(1_216_448, "reais"), "1 milhão e 216 mil reais");
        assert_eq!(money_words(2_000_000, "reais"), "2 milhões de reais");
        assert_eq!(money_words(20_000_000, "reais"), "20 milhões de reais");
        assert_eq!(money_words(1_000_000, "reais"), "um milhão de reais");
        assert_eq!(money_words(1000, "reais"), "mil reais");
        assert_eq!(money_words(2_500_000, "reais"), "2 milhões e 500 mil reais");
        assert_eq!(money_words(975_500, "reais"), "975 mil reais");
        assert_eq!(money_words(850, "reais"), "850 reais");
    }
}
