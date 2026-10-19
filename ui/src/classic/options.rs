//! The player's options (`TOptionsDlg`) and the header of a saved game that carries them together with the coach's name.
//!
//! The original keeps the options in `ELIF98.INI` (global); here they are part of [`Classic`](super::Classic) and travel in the first line of
//! the saved text (`#elifoot-classic|coach=..|stamp=..|opts=..`) so that a loaded game plays as it was left. Old saves without the header
//! load with the defaults.

/// How the end-of-round and match screens behave. The defaults are those of a fresh `ELIF98.INI` (`1020:21ea`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Options {
    /// "Mostrar chicotadas psicológicas": 0 never, 1 of human coaches only, 2 all.
    pub lashes: u8,
    /// "Gravar o jogo": 0 after every round, 1 every 3 rounds, 2 at the start of the season.
    pub save_when: u8,
    /// "Ver sorteio da taça": 0 never, 1 when there are humans, 2 always.
    pub cup_draw: u8,
    /// "Habilitar som": false is "Sim" (index 0), true is "Não" (index 1, the original's default). The port has no audio.
    pub sound_off: bool,
    /// "Tempo de jogo": index into [`PROFILES`].
    pub profile: u8,
    /// "Substituições ao intervalo".
    pub half_time_subs: bool,
    /// "Ver os desempates por penalties nos jogos sem treinadores humanos".
    pub view_computer_penalties: bool,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            lashes: 1,
            save_when: 1,
            cup_draw: 1,
            sound_off: true,
            profile: 1,
            half_time_subs: true,
            view_computer_penalties: true,
        }
    }
}

/// "Tempo de jogo": the names (`1020:21ea`'s neighbour, the table at `10c8:2c55`) and the speed of each profile. The three first are
/// always offered; the others belong to the registered versions and are not listed.
pub const PROFILES: [(&str, u32); 3] = [("Lento", 1000), ("Médio", 800), ("Rápido", 600)];

/// The values of "Gravar o jogo" and the other combos, as the dialog lists them.
pub const LASHES: [&str; 3] = ["Nunca", "Dos humanos", "Todas"];
pub const SAVE_WHEN: [&str; 3] = [
    "Todas as jornadas",
    "De 3 em 3 jornadas",
    "No início da época",
];
pub const CUP_DRAW: [&str; 3] = ["Nunca", "Quando houver humanos", "Sempre"];
pub const SOUND: [&str; 2] = ["Sim", "Não"];
pub const NO_YES: [&str; 2] = ["Não", "Sim"];

impl Options {
    /// Milliseconds one match minute lasts in the round view: the medium profile is the 90 ms of the replay, the others scale with the
    /// original's per-profile delay.
    pub fn minute_ms(&self) -> u32 {
        let d = PROFILES[(self.profile as usize).min(PROFILES.len() - 1)].1;
        (90 * d / 800).max(1)
    }

    /// Whether the game is saved after the round `round` (rounds played so far in the season), `1018:229f`.
    pub fn saves_after(&self, round: usize) -> bool {
        round == 1
            || self.save_when == 0
            || (self.save_when == 1 && round.is_multiple_of(3) && round > 0)
    }

    /// Whether the sacking of a coach is announced (`human`: the sacked coach is a human's).
    pub fn show_lashes(&self, human: bool) -> bool {
        match self.lashes {
            0 => false,
            1 => human,
            _ => true,
        }
    }

    /// Whether the cup draw is shown (`humans_in_cup`: a human's club is still in the cup).
    pub fn show_cup_draw(&self, humans_in_cup: bool) -> bool {
        match self.cup_draw {
            0 => false,
            1 => humans_in_cup,
            _ => true,
        }
    }

    fn encode(&self) -> String {
        format!(
            "{}{}{}{}{}{}{}",
            self.lashes,
            self.save_when,
            self.cup_draw,
            self.sound_off as u8,
            self.profile,
            self.half_time_subs as u8,
            self.view_computer_penalties as u8
        )
    }

    fn decode(s: &str) -> Option<Options> {
        let d: Vec<u8> = s.bytes().map(|b| b.wrapping_sub(b'0')).collect();
        if d.len() != 7 || d.iter().any(|&v| v > 9) {
            return None;
        }
        Some(Options {
            lashes: d[0].min(2),
            save_when: d[1].min(2),
            cup_draw: d[2].min(2),
            sound_off: d[3] != 0,
            profile: d[4].min(PROFILES.len() as u8 - 1),
            half_time_subs: d[5] != 0,
            view_computer_penalties: d[6] != 0,
        })
    }
}

const HEADER: &str = "#elifoot-classic";

/// What the first line of a saved text says besides the world.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Header {
    pub coach: String,
    /// `dd/mm/yy hh:mm` of the moment of saving (empty when the host gave no clock).
    pub stamp: String,
    pub opts: Option<Options>,
}

fn clean(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_control() && *c != '|' && *c != '=')
        .collect()
}

impl Header {
    pub fn line(&self, opts: &Options) -> String {
        format!(
            "{HEADER}|coach={}|stamp={}|opts={}",
            clean(&self.coach),
            clean(&self.stamp),
            opts.encode()
        )
    }

    /// Splits a saved text into its header (default when absent) and the world's text.
    pub fn split(text: &str) -> (Header, &str) {
        let mut h = Header::default();
        let mut rest = text;
        while rest.starts_with('#') {
            let (line, tail) = rest.split_once('\n').unwrap_or((rest, ""));
            if line.starts_with(HEADER) {
                for f in line.split('|').skip(1) {
                    match f.split_once('=') {
                        Some(("coach", v)) => h.coach = v.to_string(),
                        Some(("stamp", v)) => h.stamp = v.to_string(),
                        Some(("opts", v)) => h.opts = Options::decode(v),
                        _ => {}
                    }
                }
            }
            rest = tail;
        }
        (h, rest)
    }

    /// The description the game lists for a saved game: the coach on one line, the date on the next.
    pub fn description(&self) -> String {
        if self.stamp.is_empty() {
            self.coach.clone()
        } else {
            format!("{}\n{}", self.coach, self.stamp)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_header_round_trips_and_old_saves_have_none() {
        let o = Options {
            lashes: 2,
            save_when: 0,
            cup_draw: 2,
            sound_off: false,
            profile: 2,
            half_time_subs: false,
            view_computer_penalties: false,
        };
        let h = Header {
            coach: "Zé | Silva".into(),
            stamp: "17/10/26 21:57".into(),
            opts: None,
        };
        let text = format!("{}\nMAGIC\nyear|1998\n", h.line(&o));
        let (back, rest) = Header::split(&text);
        assert_eq!(rest, "MAGIC\nyear|1998\n");
        assert_eq!(back.coach, "Zé  Silva");
        assert_eq!(back.stamp, "17/10/26 21:57");
        assert_eq!(back.opts, Some(o));
        assert_eq!(back.description(), "Zé  Silva\n17/10/26 21:57");
        let (none, rest) = Header::split("MAGIC\nx\n");
        assert_eq!(none, Header::default());
        assert_eq!(rest, "MAGIC\nx\n");
    }

    #[test]
    fn autosave_follows_the_save_option() {
        let mut o = Options::default(); // every 3 rounds
        assert!(o.saves_after(1) && o.saves_after(3) && o.saves_after(6));
        assert!(!o.saves_after(2) && !o.saves_after(4));
        o.save_when = 0;
        assert!(o.saves_after(2));
        o.save_when = 2;
        assert!(o.saves_after(1) && !o.saves_after(3));
    }

    #[test]
    fn profiles_scale_the_minute() {
        let mut o = Options::default();
        assert_eq!(o.minute_ms(), 90);
        o.profile = 0;
        assert!(o.minute_ms() > 90);
        o.profile = 2;
        assert!(o.minute_ms() < 90);
    }

    #[test]
    fn lashes_and_cup_draw_levels() {
        let mut o = Options::default();
        assert!(o.show_lashes(true) && !o.show_lashes(false));
        o.lashes = 0;
        assert!(!o.show_lashes(true));
        o.lashes = 2;
        assert!(o.show_lashes(false));
        assert!(o.show_cup_draw(true) && !o.show_cup_draw(false));
    }
}
