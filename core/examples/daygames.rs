//! Prints the day's games of an `.e98` save in the order of the original's list `270a` (the order the round view shows them):
//! kind, the clubs, and each club's place in its division's list.
//! `cargo run --release -p rustfoot-core --example daygames -- game.e98`
fn main() {
    let path = std::env::args().nth(1).expect("usage: daygames game.e98");
    let save = rustfoot_core::e98::parse(&std::fs::read(path).expect("read")).expect("parse");
    let name = |id: u16| {
        save.teams
            .iter()
            .find(|t| t.id == id)
            .map_or(format!("#{id}"), |t| t.short_name.trim().to_string())
    };
    let place = |id: u16| {
        save.divisions
            .iter()
            .enumerate()
            .find_map(|(d, div)| div.1.iter().position(|&t| t == id).map(|i| (d, i)))
    };
    for (d, div) in save.divisions.iter().enumerate().take(4) {
        for &id in &div.1 {
            let t = save.teams.iter().find(|t| t.id == id).unwrap();
            println!("div {d} {:16} keys {:?}", t.short_name.trim(), t.day_keys);
        }
    }
    for g in &save.games {
        let kind = g[0];
        let h = u16::from_le_bytes([g[42], g[43]]);
        let a = u16::from_le_bytes([g[44], g[45]]);
        println!(
            "{kind} {:16} {:?}  {:16} {:?}",
            name(h),
            place(h),
            name(a),
            place(a)
        );
    }
}
