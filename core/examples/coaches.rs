//! Prints the coach records of an `.e98` save in list order: id, name, the club they coach, human, titles and score.
//! `cargo run --release -p rustfoot-core --example coaches -- game.e98`
fn main() {
    let path = std::env::args().nth(1).expect("usage: coaches game.e98");
    let save = rustfoot_core::e98::parse(&std::fs::read(path).expect("read")).expect("parse");
    for k in &save.coaches {
        let team = save.teams.iter().find(|t| t.coach == k.id);
        let club = team.map_or("-", |t| t.short_name.as_str());
        let score = team.map_or(0, |t| t.coach_score);
        println!(
            "{:4} {:24} {:20} human={} titles={:?} score={score}",
            k.id, k.name, club, k.human, k.titles
        );
    }
}
