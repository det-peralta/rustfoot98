//! `.EFT` team files (`EFa` magic). Layout verified on all 195 shipped files and against libelifoot's spec:
//! 50-byte header (`EFa` + zeros), full name, short name, background RGB+pad, foreground RGB+pad,
//! country pstr, level (1..20), squad size, pad, then `squad size` players
//! (nationality pstr, name pstr, position, pad), and finally the coach name pstr.
use crate::cipher::read_pstr;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Position {
    Goalkeeper = 0,
    Defender = 1,
    Midfielder = 2,
    Attacker = 3,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EftPlayer {
    pub nationality: String,
    pub name: String,
    pub position: Position,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EftTeam {
    pub full_name: String,
    pub short_name: String,
    pub background: [u8; 3],
    pub foreground: [u8; 3],
    pub country: String,
    pub level: u8,
    pub players: Vec<EftPlayer>,
    pub coach: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum EftError {
    BadMagic,
    Truncated,
    BadPosition(u8),
    TrailingBytes(usize),
}

pub fn parse(data: &[u8]) -> Result<EftTeam, EftError> {
    if data.len() < 3 || &data[..3] != b"EFa" {
        return Err(EftError::BadMagic);
    }
    let t = EftError::Truncated;
    let (full_name, o) = read_pstr(data, 0x32).ok_or(EftError::Truncated)?;
    let (short_name, o) = read_pstr(data, o).ok_or(EftError::Truncated)?;
    let col = data.get(o..o + 8).ok_or(EftError::Truncated)?;
    let background = [col[0], col[1], col[2]];
    let foreground = [col[4], col[5], col[6]];
    let (country, o) = read_pstr(data, o + 8).ok_or(EftError::Truncated)?;
    let meta = data.get(o..o + 3).ok_or(EftError::Truncated)?;
    let (level, count) = (meta[0], meta[1] as usize);
    let mut o = o + 3;
    let mut players = Vec::with_capacity(count);
    for _ in 0..count {
        let (nationality, n) = read_pstr(data, o).ok_or(EftError::Truncated)?;
        let (name, n) = read_pstr(data, n).ok_or(EftError::Truncated)?;
        let pos = *data.get(n).ok_or(EftError::Truncated)?;
        let position = match pos {
            0 => Position::Goalkeeper,
            1 => Position::Defender,
            2 => Position::Midfielder,
            3 => Position::Attacker,
            p => return Err(EftError::BadPosition(p)),
        };
        players.push(EftPlayer {
            nationality,
            name,
            position,
        });
        o = n + 2; // position + pad byte
    }
    let (coach, o) = read_pstr(data, o).ok_or(t)?;
    if o != data.len() {
        return Err(EftError::TrailingBytes(data.len() - o));
    }
    Ok(EftTeam {
        full_name,
        short_name,
        background,
        foreground,
        country,
        level,
        players,
        coach,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cipher::encode;

    fn build() -> Vec<u8> {
        let mut d = vec![0u8; 0x32];
        d[..3].copy_from_slice(b"EFa");
        d.extend(encode(b"CLUBE EXEMPLO"));
        d.extend(encode(b"EXEMPLO"));
        d.extend([0xff, 0xff, 0xff, 0, 0, 0, 0, 0]);
        d.extend(encode(b"POR"));
        d.extend([7, 2, 0]);
        for (n, name, pos) in [("POR", "Ze", 0u8), ("BRA", "Joao", 3)] {
            d.extend(encode(n.as_bytes()));
            d.extend(encode(name.as_bytes()));
            d.extend([pos, 0]);
        }
        d.extend(encode(b"Mister"));
        d
    }

    #[test]
    fn parses_synthetic_team() {
        let t = parse(&build()).unwrap();
        assert_eq!(t.full_name, "CLUBE EXEMPLO");
        assert_eq!(t.country, "POR");
        assert_eq!(t.level, 7);
        assert_eq!(t.players.len(), 2);
        assert_eq!(t.players[1].position, Position::Attacker);
        assert_eq!(t.coach, "Mister");
        assert_eq!(t.foreground, [0, 0, 0]);
    }

    #[test]
    fn rejects_bad_input() {
        assert_eq!(parse(b"XXX"), Err(EftError::BadMagic));
        let mut d = build();
        d.push(0);
        assert_eq!(parse(&d), Err(EftError::TrailingBytes(1)));
        d.truncate(0x40);
        assert_eq!(parse(&d), Err(EftError::Truncated));
    }

    /// Runs against the shipped files when present (they are git-ignored): all must parse exactly.
    #[test]
    fn parses_all_shipped_files_if_available() {
        let dir =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../original/elifoot98/EQUIPAS");
        let Ok(rd) = std::fs::read_dir(dir) else {
            return;
        };
        let mut n = 0;
        for e in rd.flatten() {
            let d = std::fs::read(e.path()).unwrap();
            parse(&d).unwrap_or_else(|err| panic!("{:?}: {err:?}", e.path()));
            n += 1;
        }
        assert!(n > 100);
    }
}
