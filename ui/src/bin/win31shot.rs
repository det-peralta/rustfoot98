//! Renders one of the original forms with the pixel-exact Windows 3.1 renderer into a BMP file.
//! `win31shot TABOUTDLG out.bmp [Control=text ...]`; reads `analysis/out/forms` and `analysis/out/skin` (see `tools/ui/skin.py`).
use elifoot_ui::win31::draw::{controls, dialog_frame, Ctx, DESKTOP};
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
    let (cw, ch) = (node.int("ClientWidth", 400), node.int("ClientHeight", 300));
    let bg = node.color("Color").unwrap_or(elifoot_ui::win31::fb::SILVER);
    let (ox, oy) = dialog_frame(
        &mut fb,
        &skin,
        node.str("Caption").unwrap_or(""),
        cw,
        ch,
        bg,
    );
    let fname = form.to_uppercase();
    let cx = Ctx {
        skin: &skin,
        form: &fname,
        texts: &texts,
        ox,
        oy,
        bg,
        focus: node.ident("ActiveControl").unwrap_or(""),
    };
    controls(&mut fb, &cx, &node);
    std::fs::write(&out, fb.bmp()).map_err(|e| e.to_string())?;
    println!("wrote {out}");
    Ok(())
}
