//! Prints what the importer reads from an original `.e98` save: `cargo run -p elifoot-core --example e98dump FILE`.
use elifoot_core::e98;

fn main() {
    let path = std::env::args().nth(1).expect("usage: e98dump FILE.e98");
    let bytes = std::fs::read(&path).expect("cannot read file");
    let save = e98::parse(&bytes).unwrap_or_else(|e| panic!("{e}"));
    println!(
        "{:?} year {} human team id {}",
        save.description, save.year, save.human_id
    );
    for (i, t) in save.teams.iter().enumerate() {
        println!(
            "{i:2} id {:4} {:<18} lvl {:2} pts {:2} {}-{}-{} {}:{} K {:.4} players {} opp {:?}",
            t.id,
            t.short_name,
            t.level,
            t.points,
            t.wins,
            t.draws,
            t.losses,
            t.goals_for,
            t.goals_against,
            t.k,
            t.players.len(),
            t.opponents
        );
    }
    match save.divisions() {
        Ok(d) => println!("divisions: {d:?}"),
        Err(e) => println!("divisions: {e}"),
    }
}
