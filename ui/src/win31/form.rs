//! The Delphi forms as exported by `tools/dfm.py` (JSON): a tree of controls with their properties.
use super::fb::Rgb;
use super::json::{parse, Json};
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct Node {
    pub class: String,
    pub name: String,
    pub props: BTreeMap<String, Json>,
    pub children: Vec<Node>,
}

pub type Form = Node;

impl Node {
    pub fn from_json(text: &str) -> Result<Node, String> {
        Node::from(&parse(text)?)
    }

    fn from(j: &Json) -> Result<Node, String> {
        let s = |k: &str| j.get(k).and_then(Json::as_str).unwrap_or("").to_string();
        let props = match j.get("props") {
            Some(Json::Obj(m)) => m.clone(),
            _ => BTreeMap::new(),
        };
        let children = j
            .get("children")
            .and_then(Json::as_arr)
            .unwrap_or(&[])
            .iter()
            .map(Node::from)
            .collect::<Result<_, _>>()?;
        Ok(Node {
            class: s("class"),
            name: s("name"),
            props,
            children,
        })
    }

    pub fn int(&self, k: &str, default: i32) -> i32 {
        self.props
            .get(k)
            .and_then(Json::as_i64)
            .map_or(default, |v| v as i32)
    }

    pub fn str(&self, k: &str) -> Option<&str> {
        self.props.get(k).and_then(Json::as_str)
    }

    pub fn flag(&self, k: &str, default: bool) -> bool {
        self.props.get(k).and_then(Json::as_bool).unwrap_or(default)
    }

    /// The identifier of an enumeration property (`taCenter`, `bsNone`, ...).
    pub fn ident(&self, k: &str) -> Option<&str> {
        self.props.get(k)?.get("ident")?.as_str()
    }

    /// The members of a set property (`fsBold`, ...).
    pub fn set(&self, k: &str) -> Vec<&str> {
        self.props
            .get(k)
            .and_then(|v| v.get("set"))
            .and_then(Json::as_arr)
            .map(|a| a.iter().filter_map(Json::as_str).collect())
            .unwrap_or_default()
    }

    pub fn strings(&self, k: &str) -> Vec<String> {
        self.props
            .get(k)
            .and_then(Json::as_arr)
            .map(|a| {
                a.iter()
                    .filter_map(Json::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// A colour property: either a system/Delphi identifier or a raw integer (`$00BBGGRR`).
    pub fn color(&self, k: &str) -> Option<Rgb> {
        let v = self.props.get(k)?;
        if let Some(id) = v.get("ident").and_then(Json::as_str) {
            return color_ident(id);
        }
        v.as_i64()
            .map(|n| Rgb(n as u8, (n >> 8) as u8, (n >> 16) as u8))
    }
}

/// Delphi's colour constants with the Windows 3.1 default scheme.
pub fn color_ident(id: &str) -> Option<Rgb> {
    Some(match id {
        "clBlack" | "clWindowText" | "clBtnText" | "clMenuText" => Rgb(0, 0, 0),
        "clMaroon" => Rgb(128, 0, 0),
        "clGreen" => Rgb(0, 128, 0),
        "clOlive" => Rgb(128, 128, 0),
        "clNavy" | "clActiveCaption" => Rgb(0, 0, 128),
        "clPurple" => Rgb(128, 0, 128),
        "clTeal" => Rgb(0, 128, 128),
        "clGray" | "clBtnShadow" | "clInactiveCaptionText" => Rgb(128, 128, 128),
        "clSilver" | "clBtnFace" => Rgb(192, 192, 192),
        "clRed" => Rgb(255, 0, 0),
        "clLime" => Rgb(0, 255, 0),
        "clYellow" => Rgb(255, 255, 0),
        "clBlue" => Rgb(0, 0, 255),
        "clFuchsia" => Rgb(255, 0, 255),
        "clAqua" => Rgb(0, 255, 255),
        "clWhite" | "clWindow" | "clBtnHighlight" | "clCaptionText" | "clHighlightText" => {
            Rgb(255, 255, 255)
        }
        "clHighlight" => Rgb(0, 0, 128),
        "clBackground" => Rgb(0, 128, 128),
        _ => return None,
    })
}
