//! Renders one of the original forms with the pixel-exact Windows 3.1 renderer into a BMP file.
//! `win31shot TABOUTDLG out.bmp [Control=text ...]`; reads `analysis/out/forms` and `analysis/out/skin` (see `tools/ui/skin.py`).
use elifoot_ui::win31::draw::{
    controls, dialog_frame, dialog_frame_menu, dialog_frame_thick, Ctx, DESKTOP,
};
use elifoot_ui::win31::{Fb, Node, Skin};
use std::collections::BTreeMap;
use std::path::PathBuf;

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let form = args
        .next()
        .ok_or("usage: win31shot FORM out.bmp [Control=text ...]")?;
    let out = args.next().ok_or("missing output file")?;
    let texts: BTreeMap<String, String> = args
        .filter_map(|a| {
            a.split_once('=')
                .map(|(k, v)| (k.to_string(), v.to_string()))
        })
        .collect();
    let root = PathBuf::from(std::env::var("ELIFOOT_ROOT").unwrap_or_else(|_| ".".into()));
    let skin = Skin::load(&root.join("analysis/out/skin"))?;
    let json = std::fs::read_to_string(
        root.join("analysis/out/forms")
            .join(format!("{}.json", form.to_uppercase())),
    )
    .map_err(|e| e.to_string())?;
    let node = Node::from_json(&json)?;
    let mut fb = Fb::new();
    fb.fill(0, 0, 800, 600, DESKTOP);
    let (mut cw, mut ch) = (node.int("ClientWidth", 400), node.int("ClientHeight", 300));
    if let Some((a, b)) = texts.get("@client").and_then(|v| v.split_once('x')) {
        // the program resized the form at run time
        cw = a.parse().unwrap_or(cw);
        ch = b.parse().unwrap_or(ch);
    }
    let mut bg = node.color("Color").unwrap_or(elifoot_ui::win31::fb::SILVER);
    if let Some(c) = texts.get("@color") {
        let v: Vec<u8> = c.split(',').filter_map(|p| p.parse().ok()).collect();
        if v.len() == 3 {
            bg = elifoot_ui::win31::fb::Rgb(v[0], v[1], v[2]);
        }
    }
    let menu: Vec<String> = node
        .children
        .iter()
        .filter(|c| c.class == "TMainMenu")
        .flat_map(|m| m.children.iter())
        .filter(|i| i.flag("Visible", true))
        .map(|i| i.str("Caption").unwrap_or("").to_string())
        .collect();
    let caption = texts
        .get("@caption")
        .map(String::as_str)
        .or(node.str("Caption"))
        .unwrap_or("");
    let (ox, oy) = if !menu.is_empty() {
        dialog_frame_menu(&mut fb, &skin, caption, cw, ch, bg, &menu)
    } else if node.ident("BorderStyle") == Some("bsDialog") {
        dialog_frame_thick(&mut fb, &skin, caption, cw, ch, bg)
    } else {
        dialog_frame(&mut fb, &skin, caption, cw, ch, bg)
    };
    let (ox, oy) = (
        ox + texts.get("@dx").and_then(|v| v.parse().ok()).unwrap_or(0),
        oy + texts.get("@dy").and_then(|v| v.parse().ok()).unwrap_or(0),
    );
    let lists: BTreeMap<String, Vec<String>> = texts
        .iter()
        .filter_map(|(k, v)| {
            k.strip_suffix(".list")
                .map(|n| (n.to_string(), v.split('\n').map(str::to_string).collect()))
        })
        .collect();
    let fname = form.to_uppercase();
    let cx = Ctx {
        skin: &skin,
        form: &fname,
        texts: &texts,
        ox,
        oy,
        bg,
        focus: texts
            .get("@focus")
            .map(String::as_str)
            .or(node.ident("ActiveControl"))
            .unwrap_or(""),
        lists: &lists,
        hits: &std::cell::RefCell::new(Vec::new()),
    };
    controls(&mut fb, &cx, &node);
    std::fs::write(&out, fb.bmp()).map_err(|e| e.to_string())?;
    println!("wrote {out}");
    Ok(())
}
