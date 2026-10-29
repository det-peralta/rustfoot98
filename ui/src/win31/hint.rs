//! Hints: a control whose form gives it a `Hint` and `ShowHint = True` shows the text in a hint window (`draw::hint_window`) after
//! the pointer rested on it for `PAUSE` milliseconds (Delphi 1's `Application.HintPause`). The window stays until the pointer leaves
//! the control; a click or a key hides it until then.
use super::draw::{hint_window, Hit};
use super::fb::Fb;
use super::form::Node;
use super::skin::Skin;

pub const PAUSE: u32 = 800;

#[derive(Clone, Debug, Default)]
pub struct Hover {
    /// The form and control under the pointer that have a hint, the control's `(x, y, h)` and the text.
    form: String,
    control: String,
    rect: (i32, i32, i32),
    text: String,
    ms: u32,
    shown: bool,
    cancelled: bool,
}

impl Hover {
    /// The pointer moved to `(x, y)` over the controls `hits` of the form `form` (named `name`); true when the picture changes.
    pub fn moved(&mut self, name: &str, form: Option<&Node>, hits: &[Hit], x: i32, y: i32) -> bool {
        let found = form.and_then(|f| {
            hits.iter()
                .rev()
                .filter(|h| h.contains(x, y))
                .find_map(|h| {
                    let n = f.find(&h.name)?;
                    let text = n.str("Hint").filter(|t| !t.is_empty())?;
                    (h.enabled && n.flag("ShowHint", false)).then_some((h, text))
                })
        });
        match found {
            Some((h, _)) if self.form == name && self.control == h.name => false,
            Some((h, text)) => {
                let was = self.shown;
                *self = Hover {
                    form: name.into(),
                    control: h.name.clone(),
                    rect: (h.x, h.y, h.h),
                    // the short hint: the text before a `|`
                    text: text.split('|').next().unwrap_or("").into(),
                    ..Hover::default()
                };
                was
            }
            None => self.clear(),
        }
    }

    /// Time passes; true when the hint opens.
    pub fn tick(&mut self, ms: u32) -> bool {
        if self.control.is_empty() || self.shown || self.cancelled {
            return false;
        }
        self.ms += ms;
        self.shown = self.ms >= PAUSE;
        self.shown
    }

    /// A click or a key: the hint closes until the pointer moves to another control; true when the picture changes.
    pub fn cancel(&mut self) -> bool {
        self.cancelled = true;
        std::mem::take(&mut self.shown)
    }

    /// Forgets the control (the pointer left it, or its window closed); true when the picture changes.
    pub fn clear(&mut self) -> bool {
        std::mem::take(self).shown
    }

    /// Draws the hint when it is open over the form `name` (the topmost window).
    pub fn draw(&self, fb: &mut Fb, skin: &Skin, name: &str) {
        if self.shown && self.form == name {
            hint_window(fb, skin, self.rect, &self.text);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn form() -> Node {
        Node::from_json(
            r#"{"class":"TF","name":"f","props":{},"children":[
                {"class":"TEdit","name":"e","props":{"Hint":"Nome|longo","ShowHint":true},"children":[]},
                {"class":"TEdit","name":"n","props":{"Hint":"sem"},"children":[]}]}"#,
        )
        .unwrap()
    }

    fn hit(name: &str, x: i32) -> Hit {
        Hit {
            name: name.into(),
            class: "TEdit".into(),
            x,
            y: 0,
            w: 10,
            h: 10,
            enabled: true,
        }
    }

    #[test]
    fn a_hint_opens_after_the_pause_and_closes_on_a_click() {
        let f = form();
        let hits = [hit("e", 0), hit("n", 20)];
        let mut h = Hover::default();
        h.moved("F", Some(&f), &hits, 5, 5);
        assert!(!h.tick(PAUSE - 1));
        assert!(h.tick(1));
        assert_eq!(h.text, "Nome");
        // moving inside the control keeps it; a click hides it until the pointer leaves
        assert!(!h.moved("F", Some(&f), &hits, 6, 6));
        assert!(h.cancel());
        assert!(!h.tick(PAUSE));
        // a control without `ShowHint` has none
        h.moved("F", Some(&f), &hits, 25, 5);
        assert!(!h.tick(PAUSE));
        h.moved("F", Some(&f), &hits, 5, 5);
        assert!(h.tick(PAUSE));
    }
}
