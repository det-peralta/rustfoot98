//! The records the information dialogs show: the club calendars with their results, the champions of past seasons, the history of
//! every club and every coach, the coach titles of the ranking and the options of the user's coach (the original's
//! `TCalendarEntry`, `TLastWinners`, `TTeamEvent`, `TCoachEvent` and the coach record, see `docs/season.md`).
//!
//! Everything is keyed by *names* (club names, coach names), like the rest of the world; the maps are ordered so that saving is
//! deterministic.
use crate::season::{fixtures, standings, Club};
use crate::world::{World, ROUNDS};
use std::collections::BTreeMap;

/// One line of a club's calendar (`TCalendarEntry`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fixture {
    /// Name of the opponent club.
    pub opponent: String,
    /// `+0xe`: 0 home, 1 away, 2 neutral ground.
    pub venue: u8,
    /// The club's own goals and the opponent's (`+0xa`, `+0xc`); `-1` while unplayed.
    pub goals: (i16, i16),
    /// Attendance (`+0x10`), `-1` while unplayed.
    pub gate: i64,
    /// `+7`: a cup tie.
    pub cup: bool,
    /// `+6`: 0 normal time, 1 after extra time, 2 won on penalties, 3 lost on penalties.
    pub result: u8,
    /// Round of the league (`+4`); 0 for a cup tie (shown blank).
    pub round: u16,
}

impl Fixture {
    pub fn played(&self) -> bool {
        self.goals.0 >= 0
    }
}

/// A season's champions (`TLastWinners`): the league champion and its coach, the cup winner and its coach.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Winner {
    pub year: u16,
    pub champion: String,
    pub champion_coach: String,
    pub cup_winner: String,
    pub cup_coach: String,
}

/// An entry of a club's history (`TTeamEvent`): `kind` 1 league title, 2 cup, 3 double, 4 second place, 5 third place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClubEvent {
    pub kind: u8,
    pub year: u32,
    /// The coach at the time.
    pub coach: String,
}

/// An entry of a coach's history (`TCoachEvent`): 1 hired by a club, 2 sacked, 3 league title, 4 cup, 5 joined a club (new game),
/// 6 relegated to a club's bench (coach swap, worse club), 7 promoted to a club, 8 double, 9 second place, 10 third place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoachEvent {
    pub kind: u8,
    pub year: u32,
    /// The club of the event (empty for "sacked").
    pub club: String,
}

/// The profile of the user's coach (`TCoachOptionsDlg`, five bytes at `coach+0x1a`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CoachOptions {
    /// The computer manages the wages of the domestic players, of the foreigners covered by the Bosman rule and of the other
    /// foreigners (renegotiation after a round, `World::club_after_round`).
    pub salary: [bool; 3],
    /// Auction offers: 0 bid for every player, 1 not for the weaker ones, 2 do not buy in auctions (the user gets no listing).
    pub auction: u8,
    /// "I do not want the others to see my offers": only a display option, the computer clubs never look at the user's offers.
    pub encrypt: bool,
}

impl Default for CoachOptions {
    /// The port always let the computer renegotiate the user's wages (the original asks the user in a dialog), so that is the default;
    /// the original's new-game values are `[false; 3]`, auction 1, encrypt off.
    fn default() -> Self {
        CoachOptions {
            salary: [true; 3],
            auction: 1,
            encrypt: false,
        }
    }
}

/// A line of the coach ranking.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoachRank {
    pub coach: String,
    pub club: String,
    /// The user's coach.
    pub human: bool,
    /// League and cup titles.
    pub titles: (u16, u16),
    pub score: i64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct History {
    pub winners: Vec<Winner>,
    /// Calendars by club name; a club without an entry has not played this season (its list is generated from the schedule).
    pub calendars: BTreeMap<String, Vec<Fixture>>,
    pub club_events: BTreeMap<String, Vec<ClubEvent>>,
    pub coach_events: BTreeMap<String, Vec<CoachEvent>>,
    /// League and cup titles per coach name (`coach+0x16`, `+0x18`), the first keys of the coach ranking.
    pub coach_titles: BTreeMap<String, (u16, u16)>,
    pub options: CoachOptions,
    /// The user's coach is on holiday: the computer plays for his club (`coach+6`).
    pub holiday: bool,
}

/// Where a cup tie goes in a calendar: right after the league game of the round it follows.
pub fn insert_cup(list: &mut Vec<Fixture>, after_round: usize, f: Fixture) {
    let pos = list
        .iter()
        .rposition(|x| !x.cup && x.round as usize <= after_round)
        .map_or(0, |p| p + 1);
    // after earlier cup ties of the same round
    let mut at = pos;
    while at < list.len() && list[at].cup {
        at += 1;
    }
    list.insert(at, f);
}

impl History {
    pub fn add_club_event(&mut self, club: &str, kind: u8, year: u32, coach: &str) {
        self.club_events
            .entry(club.to_string())
            .or_default()
            .push(ClubEvent {
                kind,
                year,
                coach: coach.to_string(),
            });
    }

    pub fn add_coach_event(&mut self, coach: &str, kind: u8, year: u32, club: &str) {
        self.coach_events
            .entry(coach.to_string())
            .or_default()
            .push(CoachEvent {
                kind,
                year,
                club: club.to_string(),
            });
    }

    /// The end of a season (`1020:038e`): the champions list, the titles of club and coach, second and third place of the first
    /// division. `top3` are the first three clubs of the first division with their coaches.
    pub fn season_end(
        &mut self,
        year: u32,
        top3: &[(String, String)],
        cup: Option<(String, String)>,
    ) {
        let Some((champion, champion_coach)) = top3.first().cloned() else {
            return;
        };
        let (cup_winner, cup_coach) = cup.clone().unwrap_or_default();
        self.winners.push(Winner {
            year: year as u16,
            champion: champion.clone(),
            champion_coach: champion_coach.clone(),
            cup_winner: cup_winner.clone(),
            cup_coach: cup_coach.clone(),
        });
        self.coach_titles
            .entry(champion_coach.clone())
            .or_default()
            .0 += 1;
        if cup.is_some() {
            self.coach_titles.entry(cup_coach.clone()).or_default().1 += 1;
        }
        if cup.is_some() && champion == cup_winner {
            self.add_club_event(&champion, 3, year, &champion_coach);
            self.add_coach_event(&champion_coach, 8, year, &champion);
        } else {
            self.add_club_event(&champion, 1, year, &champion_coach);
            self.add_coach_event(&champion_coach, 3, year, &champion);
            if cup.is_some() {
                self.add_club_event(&cup_winner, 2, year, &cup_coach);
                self.add_coach_event(&cup_coach, 4, year, &cup_winner);
            }
        }
        for (k, (club, coach)) in top3.iter().enumerate().skip(1).take(2) {
            self.add_club_event(club, 3 + k as u8, year, coach);
            self.add_coach_event(coach, 8 + k as u8, year, club);
        }
    }
}

impl World {
    fn club_named(&self, name: &str) -> Option<&Club> {
        self.divisions
            .iter()
            .flatten()
            .chain(self.pool.iter())
            .find(|c| c.name == name)
    }

    /// The league calendar of the season for a club of the four divisions with nothing played yet (the 14 rounds of the schedule).
    fn fresh_calendar(&self, club: &str) -> Vec<Fixture> {
        let Some((d, i)) = self
            .divisions
            .iter()
            .enumerate()
            .find_map(|(d, v)| v.iter().position(|c| c.name == club).map(|i| (d, i)))
        else {
            return Vec::new();
        };
        (1..=ROUNDS)
            .filter_map(|r| {
                fixtures(r).into_iter().find_map(|(h, a)| {
                    let (other, venue) = if h == i {
                        (a, 0)
                    } else if a == i {
                        (h, 1)
                    } else {
                        return None;
                    };
                    Some(Fixture {
                        opponent: self.divisions[d][other].name.clone(),
                        venue,
                        goals: (-1, -1),
                        gate: -1,
                        cup: false,
                        result: 0,
                        round: r as u16,
                    })
                })
            })
            .collect()
    }

    /// `1020:0148`: at the start of a new game every coach joins his club (event 5).
    pub(crate) fn log_new_game(&mut self) {
        let year = self.year as u32;
        let joins: Vec<(String, String)> = self
            .divisions
            .iter()
            .flatten()
            .chain(self.pool.iter())
            .filter(|c| !c.coach_name.is_empty())
            .map(|c| (c.coach_name.clone(), c.name.clone()))
            .collect();
        for (coach, club) in joins {
            self.history.add_coach_event(&coach, 5, year, &club);
        }
    }

    /// The calendar of a club as the list of `Calendário` shows it: its fixtures of the league with the results so far, and the
    /// cup ties it has played this season after the round they followed.
    pub fn calendar(&self, club: &str) -> Vec<Fixture> {
        self.history
            .calendars
            .get(club)
            .cloned()
            .unwrap_or_else(|| self.fresh_calendar(club))
    }

    /// Writes the results of the round just played into the calendars of the clubs (`1010:9396` fills the entries).
    pub(crate) fn record_league_fixtures(&mut self, round: usize) {
        let mut results: Vec<(String, Fixture)> = Vec::new();
        for (d, r) in &self.last_round {
            let (h, a) = (&self.divisions[*d][r.home], &self.divisions[*d][r.away]);
            let mk = |opp: &Club, venue: u8, own: i32, other: i32| Fixture {
                opponent: opp.name.clone(),
                venue,
                goals: (own as i16, other as i16),
                gate: r.attendance,
                cup: false,
                result: 0,
                round: round as u16,
            };
            results.push((h.name.clone(), mk(a, 0, r.home_goals, r.away_goals)));
            results.push((a.name.clone(), mk(h, 1, r.away_goals, r.home_goals)));
        }
        for (club, f) in results {
            let mut list = self.calendar(&club);
            if let Some(e) = list.iter_mut().find(|e| !e.cup && e.round == f.round) {
                *e = f;
            }
            self.history.calendars.insert(club, list);
        }
    }

    /// Adds the cup ties of the last cup round to the calendars of both clubs.
    pub(crate) fn record_cup_fixtures(&mut self, round: usize) {
        let Some(cr) = self.last_cup.clone() else {
            return;
        };
        for m in &cr.matches {
            for (me, other, own, against) in [
                (&m.home, &m.away, m.home_goals, m.away_goals),
                (&m.away, &m.home, m.away_goals, m.home_goals),
            ] {
                let result = if m.shootout {
                    if m.winner == *me {
                        2
                    } else {
                        3
                    }
                } else {
                    m.extra_time as u8
                };
                let f = Fixture {
                    opponent: other.clone(),
                    venue: (me != &m.home) as u8,
                    goals: (own as i16, against as i16),
                    gate: m.attendance,
                    cup: true,
                    result,
                    round: 0,
                };
                let mut list = self.calendar(me);
                insert_cup(&mut list, round, f);
                self.history.calendars.insert(me.clone(), list);
            }
        }
    }

    /// `1020:038e`: the champions list and the titles at the end of the season, before the clubs change division.
    pub(crate) fn record_season_history(&mut self) {
        let top3: Vec<(String, String)> = standings(&self.divisions[0])
            .into_iter()
            .take(3)
            .map(|i| {
                let c = &self.divisions[0][i];
                (c.name.clone(), c.coach_name.clone())
            })
            .collect();
        let cup = self.cup_champion.clone().map(|n| {
            let coach = self
                .club_named(&n)
                .map(|c| c.coach_name.clone())
                .unwrap_or_default();
            (n, coach)
        });
        let year = self.year as u32;
        self.history.season_end(year, &top3, cup);
    }

    /// The coach swap of a sacking (`1010:4f40`): the sacked club's coach goes to the other club (event 6) and the other club's
    /// coach takes over (event 7).
    pub(crate) fn log_coach_swap(&mut self, sacked: &Club, picked: &Club) {
        let year = self.year as u32;
        self.history
            .add_coach_event(&sacked.coach_name, 6, year, &picked.name);
        self.history
            .add_coach_event(&picked.coach_name, 7, year, &sacked.name);
    }

    /// `1050:1d59` applied with the bubble sort `1068:0e0d` (stable): the coaches by league titles, cup titles and score,
    /// best first.
    pub fn coach_ranking(&self) -> Vec<CoachRank> {
        let mut v: Vec<CoachRank> = self
            .divisions
            .iter()
            .flatten()
            .chain(self.pool.iter())
            .map(|c| CoachRank {
                coach: c.coach_name.clone(),
                club: c.name.clone(),
                human: self.human.as_ref().is_some_and(|h| h.club == c.name),
                titles: self
                    .history
                    .coach_titles
                    .get(&c.coach_name)
                    .copied()
                    .unwrap_or_default(),
                score: c.coach_score,
            })
            .collect();
        v.sort_by(|a, b| {
            b.titles
                .0
                .cmp(&a.titles.0)
                .then(b.titles.1.cmp(&a.titles.1))
                .then(b.score.cmp(&a.score))
        });
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fx(round: u16, cup: bool) -> Fixture {
        Fixture {
            opponent: "X".into(),
            venue: 0,
            goals: (-1, -1),
            gate: -1,
            cup,
            result: 0,
            round,
        }
    }

    #[test]
    fn cup_ties_follow_their_league_round() {
        let mut l: Vec<Fixture> = (1..=4).map(|r| fx(r, false)).collect();
        insert_cup(&mut l, 2, fx(0, true));
        insert_cup(&mut l, 4, fx(0, true));
        let rounds: Vec<(u16, bool)> = l.iter().map(|f| (f.round, f.cup)).collect();
        assert_eq!(
            rounds,
            [
                (1, false),
                (2, false),
                (0, true),
                (3, false),
                (4, false),
                (0, true)
            ]
        );
    }

    #[test]
    fn calendars_fill_up_and_the_season_end_writes_the_champions() {
        let mut w = World::synthetic(5);
        let club = w.human.as_ref().unwrap().club.clone();
        assert_eq!(w.calendar(&club).len(), 14, "the schedule before any game");
        assert!(w.calendar(&club).iter().all(|f| !f.played() && !f.cup));
        w.play_round();
        w.play_round();
        let cal = w.calendar(&club);
        assert!(cal[0].played() && cal[1].played() && !cal[2].played());
        let mine = &w.divisions[w.human_location().unwrap().0][w.human_location().unwrap().1];
        let gf: i32 = cal
            .iter()
            .filter(|f| f.played() && !f.cup)
            .map(|f| f.goals.0 as i32)
            .sum();
        assert_eq!(gf, mine.goals_for, "the calendar agrees with the standings");
        // a saved game keeps the history
        let back = World::load(&w.save()).unwrap();
        assert_eq!(back.history, w.history);
        // the cup ties are in the list too, after their round
        assert!(w.calendar(&club).iter().filter(|f| f.cup).count() <= 1);
        w.play_season();
        assert_eq!(w.history.winners.len(), 1);
        let champion = &w.last_season.as_ref().unwrap().tables[0][0].name;
        assert_eq!(&w.history.winners[0].champion, champion);
        assert!(w.history.club_events.contains_key(champion));
        assert!(
            w.history.calendars.is_empty(),
            "a new season starts with fresh calendars"
        );
        assert_eq!(w.coach_ranking().len(), 38);
        let first = &w.coach_ranking()[0];
        assert!(
            first.titles.0 + first.titles.1 > 0,
            "the coach with titles heads the ranking"
        );
    }

    fn rich_world(seed: u32) -> World {
        let mut w = World::synthetic(seed);
        let (d, i) = w.human_location().unwrap();
        w.divisions[d][i].fin.cash = 900_000_000;
        w
    }

    #[test]
    fn the_coach_profile_decides_which_wages_the_computer_renegotiates() {
        let fresh_after = |salary: [bool; 3]| {
            let mut w = rich_world(12);
            w.history.options.salary = salary;
            let (d, i) = w.human_location().unwrap();
            for s in w.divisions[d][i].slots.iter_mut() {
                s.wage *= 10; // everybody overpaid
                s.foreign = 0;
            }
            for _ in 0..4 {
                w.play_round();
            }
            let (d, i) = w.human_location().unwrap();
            w.divisions[d][i].slots.iter().filter(|s| s.fresh).count()
        };
        assert!(fresh_after([true, true, true]) > 0);
        assert_eq!(
            fresh_after([false, true, true]),
            0,
            "domestic players are left alone"
        );
        assert!(fresh_after([false, false, false]) == 0);
    }

    #[test]
    fn not_buying_in_auctions_leaves_the_user_no_listing() {
        let listings = |mode: u8| {
            let mut w = rich_world(9);
            w.history.options.auction = mode;
            (0..14)
                .map(|_| {
                    w.play_round();
                    w.board.len()
                })
                .sum::<usize>()
        };
        assert!(
            listings(1) > 0,
            "the default profile gets auctions to bid in"
        );
        assert_eq!(listings(2), 0);
    }

    #[test]
    fn a_coach_on_holiday_lets_the_computer_play() {
        let mut w = rich_world(13);
        w.history.holiday = true;
        w.play_round(); // plays with the automatic line-up, nothing to break
        assert_eq!(w.round, 1);
        let back = World::load(&w.save()).unwrap();
        assert!(back.history.holiday);
        assert_eq!(back.history.options, w.history.options);
    }

    #[test]
    fn a_double_is_one_event_and_places_two_and_three_are_recorded() {
        let mut h = History::default();
        let top: Vec<(String, String)> = [("A", "ca"), ("B", "cb"), ("C", "cc")]
            .iter()
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .collect();
        h.season_end(2000, &top, Some(("A".into(), "ca".into())));
        assert_eq!(h.winners.len(), 1);
        assert_eq!(h.club_events["A"][0].kind, 3);
        assert_eq!(h.coach_events["ca"][0].kind, 8);
        assert_eq!(h.club_events["B"][0].kind, 4);
        assert_eq!(h.club_events["C"][0].kind, 5);
        assert_eq!(h.coach_events["cc"][0].kind, 10);
        assert_eq!(h.coach_titles["ca"], (1, 1));
        let mut h = History::default();
        h.season_end(2001, &top, Some(("C".into(), "cc".into())));
        assert_eq!(h.club_events["A"][0].kind, 1);
        let kinds: Vec<u8> = h.club_events["C"].iter().map(|e| e.kind).collect();
        assert_eq!(kinds, [2, 5]);
    }
}
