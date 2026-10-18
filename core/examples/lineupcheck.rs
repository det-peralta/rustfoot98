use elifoot_core::e98;
use elifoot_core::lineup;
use elifoot_core::sector::{Slot, Status};

fn main() {
    let save = e98::parse(&std::fs::read(std::env::args().nth(1).unwrap()).unwrap()).unwrap();
    let divs = save.divisions().unwrap();
    let (mut ok, mut bad, mut skipped) = (0, 0, 0);
    for idx in divs.iter() {
        for &i in idx {
            let t = &save.teams[i];
            if t.players.iter().all(|p| p.status == 0) {
                skipped += 1;
                continue;
            }
            let mut slots: Vec<Slot> = t
                .players
                .iter()
                .map(|p| {
                    let mut s = Slot::new(&p.name, p.position, p.strength, true, Status::Out);
                    s.goals = p.goals;
                    s.ban = p.ban;
                    s.injured = p.injured;
                    s
                })
                .collect();
            let want: Vec<u8> = t.players.iter().map(|p| p.status).collect();
            lineup::default_statuses(&mut slots);
            let got: Vec<u8> = slots
                .iter()
                .map(|s| match s.status {
                    Status::Out => 0,
                    Status::Starter => 1,
                    Status::Sub => 2,
                })
                .collect();
            if got == want {
                ok += 1
            } else {
                bad += 1;
                println!(
                    "{}: want {:?}\n   got  {:?}\n   str  {:?} goals {:?} pos {:?}",
                    t.short_name,
                    want,
                    got,
                    t.players.iter().map(|p| p.strength).collect::<Vec<_>>(),
                    t.players.iter().map(|p| p.goals).collect::<Vec<_>>(),
                    t.players.iter().map(|p| p.position).collect::<Vec<_>>()
                );
            }
        }
    }
    println!("ok {ok} bad {bad} skipped {skipped}");
}
