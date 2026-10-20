use rustfoot_core::e98;
use std::collections::HashMap;
fn main() {
    let a = e98::parse(&std::fs::read(std::env::args().nth(1).unwrap()).unwrap()).unwrap();
    let b = e98::parse(&std::fs::read(std::env::args().nth(2).unwrap()).unwrap()).unwrap();
    let mut owner: HashMap<u16, (String, u16, u8)> = HashMap::new();
    for t in &a.teams {
        for p in &t.players {
            owner.insert(p.id, (t.short_name.clone(), p.strength, p.position));
        }
    }
    for t in &b.teams {
        println!(
            "{:<14} {} -> {} players",
            t.short_name,
            a.teams
                .iter()
                .find(|x| x.id == t.id)
                .map(|x| x.players.len())
                .unwrap_or(0),
            t.players.len()
        );
        for p in &t.players {
            match owner.get(&p.id) {
                Some((o, s, pos)) if *o != t.short_name => println!(
                    "   MOVED {} (str {}->{}, pos {}) from {}",
                    p.name, s, p.strength, pos, o
                ),
                None => println!("   NEW {} str {} pos {}", p.name, p.strength, p.position),
                _ => {}
            }
        }
    }
}
