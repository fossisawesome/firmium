
/// Open a save dialog and write `contents` to the chosen path. Returns
/// `Ok(false)` if the user cancelled the dialog.
pub(crate) async fn save_export(default_name: String, ext: String, contents: String) -> Result<bool, String> {
    let handle = rfd::AsyncFileDialog::new()
        .set_file_name(&default_name)
        .add_filter("export", &[ext.as_str()])
        .save_file()
        .await;
    let Some(handle) = handle else { return Ok(false) };
    crate::commands::stats::save_text_file(handle.path().to_string_lossy().to_string(), contents)?;
    Ok(true)
}
