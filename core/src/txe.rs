//! `.TXE` text tables (`COUNTRY.TXE`, `REFEREE.TXE`, `CTRGROUP/*.TXE`): a sequence of double-encoded
//! Pascal strings. The first entry is the table title (e.g. `Bosman`), the rest are the rows.
use crate::cipher::read_pstr_n;

pub fn parse(data: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let mut o = 0;
    while o < data.len() {
        match read_pstr_n(data, o, true) {
            Some((s, next)) => {
                out.push(s);
                o = next;
            }
            None => break,
        }
    }
    out
}
