//! Checks that the per-round order keys of a save's clubs (`team+0x22..`, see `season::DAY_ORDER`) depend only on the club's
//! slot in the schedule, i.e. equal `DAY_ORDER[slot]` for every club of the four divisions.
//! `cargo run --release -p rustfoot-core --example daykeys -- game.e98`
fn main() {
    let path = std::env::args().nth(1).expect("usage: daykeys game.e98");
    let save = rustfoot_core::e98::parse(&std::fs::read(path).expect("read")).expect("parse");
    let w = rustfoot_core::world::World::from_e98(1, &save).expect("world");
    let (mut ok, mut bad) = (0, 0);
    for (d, div) in w.divisions.iter().enumerate() {
        for (slot, c) in div.iter().enumerate() {
            let t = save
                .teams
                .iter()
                .find(|t| t.short_name.trim() == c.short_name.trim())
                .expect("team");
            if t.day_keys == rustfoot_core::season::DAY_ORDER[slot] {
                ok += 1;
            } else {
                bad += 1;
                println!("div {d} slot {slot} {}: {:?}", c.name, t.day_keys);
            }
        }
    }
    println!("{ok} clubs match DAY_ORDER, {bad} do not");
}
