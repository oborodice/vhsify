use std::sync::atomic::{AtomicBool, Ordering};

pub(crate) const MESSAGE: &str = "Interrupted";
// 128 + SIGINT(2)。シグナルで中断されたコマンドの慣習的な終了コード
pub(crate) const EXIT_CODE: i32 = 130;

static REQUESTED: AtomicBool = AtomicBool::new(false);

// Ctrl+C でその場で終了すると Drop が実行されず一時ディレクトリが残るため、中断の印だけを立てて
// 処理の区切りでエラーとして戻る。2 回目の Ctrl+C では後片付けを待たずに終了する
pub(crate) fn watch() {
    ctrlc::set_handler(|| {
        if REQUESTED.swap(true, Ordering::Relaxed) {
            std::process::exit(EXIT_CODE);
        }
    })
    .expect("Failed to set Ctrl+C handler");
}

pub(crate) fn requested() -> bool {
    REQUESTED.load(Ordering::Relaxed)
}

pub(crate) fn check() -> Result<(), String> {
    if requested() {
        return Err(MESSAGE.to_string());
    }
    Ok(())
}
