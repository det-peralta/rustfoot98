//! Renders the round view with the content of a capture of the original (`analysis/shots/match12.png`) for comparing the layout:
//! `round_shot out.bmp [minute]`. Needs the skin pack in `analysis/out/skin`.
use elifoot_core::matchsim::EventKind;
use elifoot_ui::classic::round_view::{draw, Club, Ev, Game, RoundView};
use elifoot_ui::win31::fb::Rgb;
use elifoot_ui::win31::{Fb, Skin};

type Row = (&'static str, i64, Rgb, Rgb, &'static str, Rgb, Rgb, &'static [(u16, usize, char, &'static str)]);

const ROWS: [Row; 16] = [
    ("CORINTHIANS", 52294, Rgb(248,252,248), Rgb(0,0,0), "LIERSE", Rgb(248,252,0), Rgb(0,0,0), &[(37,0,'g',"Mirandinha")]),
    ("BOTAFOGO", 52520, Rgb(0,0,0), Rgb(248,252,248), "MALINES", Rgb(128,0,0), Rgb(248,252,0), &[(15,1,'g',"Djair"),(22,0,'g',"Dimba")]),
    ("SÃO PAULO", 58127, Rgb(248,0,0), Rgb(248,252,248), "SANTOS", Rgb(0,0,0), Rgb(248,252,248), &[(29,1,'g',"Anderson")]),
    ("PALMEIRAS", 53867, Rgb(0,128,0), Rgb(248,252,248), "FLAMENGO", Rgb(248,0,0), Rgb(0,0,0), &[(18,0,'g',"Leandro")]),
    ("ST LIEGE", 50000, Rgb(248,252,248), Rgb(248,0,0), "INTERNACIONAL", Rgb(248,0,0), Rgb(248,252,248), &[]),
    ("EENDRACHT AALST", 27401, Rgb(192,192,192), Rgb(0,0,0), "JUVENTUDE", Rgb(0,128,0), Rgb(248,252,248), &[]),
    ("LUSA", 35091, Rgb(0,252,0), Rgb(248,0,0), "COLO COLO", Rgb(248,252,248), Rgb(0,0,0), &[(44,1,'r',"Ricardo Miranda")]),
    ("ATLETICO MG", 36510, Rgb(0,0,0), Rgb(248,252,0), "VASCO DA GAMA", Rgb(248,252,248), Rgb(0,0,0), &[(9,1,'i',"Marques")]),
    ("PARANA", 21787, Rgb(248,0,0), Rgb(0,0,128), "CORITIBA", Rgb(0,128,0), Rgb(248,252,248), &[]),
    ("AVAÍ", 20000, Rgb(0,0,248), Rgb(248,252,248), "BRUSQUE", Rgb(128,0,0), Rgb(0,128,0), &[(41,0,'g',"Chulapa")]),
    ("MOLENBEEK", 28919, Rgb(192,192,192), Rgb(248,0,0), "PONTE PRETA", Rgb(0,0,0), Rgb(248,252,248), &[(35,1,'g',"Grizzo")]),
    ("GREMIO", 17539, Rgb(0,0,128), Rgb(248,252,248), "FLUMINENSE", Rgb(248,0,0), Rgb(0,252,0), &[]),
    ("KSV WAREGEM", 9849, Rgb(0,0,0), Rgb(248,252,248), "CEARÁ", Rgb(0,0,0), Rgb(248,252,248), &[]),
    ("GOIAS", 4785, Rgb(0,128,0), Rgb(248,252,248), "GUARANI", Rgb(0,0,0), Rgb(0,252,0), &[]),
    ("SPORT", 11020, Rgb(0,0,0), Rgb(248,0,0), "FIGUEIRENSE", Rgb(0,0,0), Rgb(248,252,248), &[]),
    ("CRUZEIRO", 7993, Rgb(0,0,128), Rgb(248,252,248), "BLUMENAU", Rgb(0,128,0), Rgb(128,0,0), &[(40,1,'g',"Cláudio")]),
];

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let out = args.get(1).ok_or("usage: round_shot out.bmp [minute]")?;
    let minute: u16 = args.get(2).and_then(|m| m.parse().ok()).unwrap_or(72);
    let skin = Skin::load(std::path::Path::new("analysis/out/skin"))?;
    let mut divisions: [Vec<Game>; 4] = Default::default();
    for (i, r) in ROWS.iter().enumerate() {
        let events = r
            .7
            .iter()
            .map(|&(minute, side, k, p)| Ev {
                minute,
                side,
                kind: match k {
                    'r' => EventKind::RedCard,
                    'i' => EventKind::Injury,
                    _ => EventKind::Goal,
                },
                player: p.to_string(),
            })
            .collect();
        divisions[i / 4].push(Game {
            attendance: r.1,
            clubs: [
                Club { name: r.0.into(), colors: (r.2, r.3) },
                Club { name: r.4.into(), colors: (r.5, r.6) },
            ],
            events,
            human: false,
        });
    }
    let view = RoundView { title: "3ª Jornada - 2026".into(), divisions, minute, acc_ms: 0, halftime_seen: false };
    let mut fb = Fb::new();
    draw(&mut fb, &skin, &view);
    std::fs::write(out, fb.bmp()).map_err(|e| e.to_string())?;
    Ok(())
}
