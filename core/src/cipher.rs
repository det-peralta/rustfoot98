//! String cipher used by every Elifoot data file.
//! A Pascal string `[len][bytes]` where `enc[i] = enc[i-1] + plain[i]` (mod 256) and `enc[-1] = len`.
//! `.TXE` files apply the transform twice.

/// Decodes the body of a string (without the length byte) given the seed (the length).
pub fn decode_body(body: &[u8], seed: u8) -> Vec<u8> {
    let mut prev = seed;
    body.iter()
        .map(|&c| {
            let p = c.wrapping_sub(prev);
            prev = c;
            p
        })
        .collect()
}

/// Encodes a plain string body into the delta cipher, returning `[len][encoded...]`.
pub fn encode(plain: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(plain.len() + 1);
    out.push(plain.len() as u8);
    for &p in plain {
        let prev = *out.last().unwrap();
        out.push(prev.wrapping_add(p));
    }
    out
}

/// Reads a (single-encoded) Pascal string at `offset`; returns the text (Latin-1) and the next offset.
pub fn read_pstr(data: &[u8], offset: usize) -> Option<(String, usize)> {
    read_pstr_n(data, offset, false)
}

/// Same, but with the double encoding used by `.TXE` files when `twice` is true.
pub fn read_pstr_n(data: &[u8], offset: usize, twice: bool) -> Option<(String, usize)> {
    let n = *data.get(offset)? as usize;
    let body = data.get(offset + 1..offset + 1 + n)?;
    let mut plain = decode_body(body, n as u8);
    if twice {
        plain = decode_body(&plain, n as u8);
    }
    Some((plain.iter().map(|&b| b as char).collect(), offset + 1 + n))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn documented_example() {
        // from libelifoot's format document: 'elf98' -> 056ad63c75ad
        assert_eq!(encode(b"elf98"), [0x05, 0x6a, 0xd6, 0x3c, 0x75, 0xad]);
        assert_eq!(
            read_pstr(&encode(b"elf98"), 0).unwrap(),
            ("elf98".to_string(), 6)
        );
    }

    #[test]
    fn real_player_name() {
        // "POR" as stored in team files
        assert_eq!(read_pstr(&[0x03, 0x53, 0xa2, 0xf4], 0).unwrap().0, "POR");
    }

    #[test]
    fn txe_double() {
        let double = {
            let once = encode(b"Bosman");
            let body = decode_body(&once[1..], 0); // not used; build double manually below
            let _ = body;
            // encode twice: apply delta transform to the encoded body with the same seed
            let n = once[0];
            let mut prev = n;
            let mut out = vec![n];
            for &c in &once[1..] {
                let v = prev.wrapping_add(c);
                out.push(v);
                prev = v;
            }
            out
        };
        assert_eq!(read_pstr_n(&double, 0, true).unwrap().0, "Bosman");
    }
}
