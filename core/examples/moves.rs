//! Players who changed clubs between two saves of the original: `cargo run --release -p rustfoot-core --example moves -- a.e98 b.e98`.
use rustfoot_core::e98;

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [a, b] = &args[..] else {
        return Err("usage: moves before.e98 after.e98".into());
    };
    let before = e98::parse(&std::fs::read(a).map_err(|e| e.to_string())?)?;
    let after = e98::parse(&std::fs::read(b).map_err(|e| e.to_string())?)?;
    let club_of = |s: &e98::E98, id: u16| {
        s.teams
            .iter()
            .find(|t| t.players.iter().any(|p| p.id == id))
            .map(|t| t.short_name.clone())
    };
    for t in &before.teams {
        for p in &t.players {
            let now = club_of(&after, p.id);
            if now.as_deref() != Some(t.short_name.as_str()) {
                let q = after
                    .teams
                    .iter()
                    .flat_map(|t| &t.players)
                    .find(|q| q.id == p.id);
                println!(
                    "{:<20} {:<16} str {:>2} wage {:>6} -> {:<16} {}",
                    p.name,
                    t.short_name,
                    p.strength,
                    p.wage,
                    now.unwrap_or_else(|| "(gone)".into()),
                    q.map(|q| format!("str {} wage {} fresh {}", q.strength, q.wage, q.fresh))
                        .unwrap_or_default()
                );
            }
        }
    }
    for t in &after.teams {
        for p in &t.players {
            if club_of(&before, p.id).is_none() {
                println!(
                    "{:<20} new at {:<16} str {:>2} wage {}",
                    p.name, t.short_name, p.strength, p.wage
                );
            }
        }
    }
    println!(
        "transfers listed in the save: {} (before: {})",
        after.transfers.len(),
        before.transfers.len()
    );
    for t in after.transfers.iter().skip(before.transfers.len()) {
        println!("  new: {t:?}");
    }
    Ok(())
}
