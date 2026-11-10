use crate::types::Widget;
use anyhow::Result;
use std::fs::File;
use std::path::PathBuf;

pub fn get_widgets(widgets_path: PathBuf) -> Result<Vec<Widget>> {
    let mut widgets = Vec::new();

    for entry in widgets_path.read_dir()? {
        let entry = entry?;
        let manifest_path = entry.path().join("manifest.json");

        if manifest_path.exists() {
            let mut widget: Widget = serde_json::from_reader(File::open(&manifest_path)?)?;

            widget.path = entry.path();
            widget.preview_path = widget.preview_path.map(|x| entry.path().join(x));
            widgets.push(widget);
        }
    }

    Ok(widgets)
}
