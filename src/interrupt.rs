use std::sync::atomic::{AtomicBool, Ordering};

pub(crate) const MESSAGE: &str = "Interrupted";
// 128 + SIGINT(2)。シグナルで中断されたコマンドの慣習的な終了コード
pub(crate) const EXIT_CODE: i32 = 130;

static REQUESTED: AtomicBool = AtomicBool::new(false);

// Ctrl+C でその場で終了すると Drop が実行されず一時ディレクトリが残るため、中断の印だけを立てて
// 処理の区切りでエラーとして戻る。2 回目の Ctrl+C では後片付けを待たずに終了する
pub(crate) fn watch() {
    watch_platform().expect("Failed to set Ctrl+C handler");
}

// ターミナルの Ctrl+C は ffmpeg にも届き、ffmpeg はすぐに終わることがある。印を別のスレッドで立てると、
// ffmpeg の失敗を受け取って main に戻るまでに間に合わず、中断ではなくエラーとして報告してしまう。
// そのため Unix では、シグナルを受けたその場(シグナルハンドラの中)で印を立てる
#[cfg(unix)]
fn watch_platform() -> std::io::Result<()> {
    use signal_hook::consts::SIGINT;
    use signal_hook::flag;

    // 印がすでに立っていれば終了する処理を、印を立てる処理より先に登録する(登録順に実行される)
    flag::register_conditional_shutdown(SIGINT, EXIT_CODE, &REQUESTED)?;
    flag::register(SIGINT, &REQUESTED)?;
    Ok(())
}

#[cfg(not(unix))]
fn watch_platform() -> Result<(), ctrlc::Error> {
    ctrlc::set_handler(|| {
        if REQUESTED.swap(true, Ordering::Relaxed) {
            std::process::exit(EXIT_CODE);
        }
    })
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
