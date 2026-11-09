use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

pub(crate) const MESSAGE: &str = "Interrupted";

static REQUESTED: AtomicBool = AtomicBool::new(false);
// 受けたシグナルの番号。Windows では Ctrl+C しか扱わないので、SIGINT(2) のまま変わらない
static SIGNAL: AtomicUsize = AtomicUsize::new(2);

// Ctrl+C・kill(SIGTERM)・ターミナルを閉じたとき(SIGHUP)にその場で終了すると Drop が実行されず
// 一時ディレクトリが残るため、中断の印だけを立てて処理の区切りでエラーとして戻る。2 回目のシグナルでは、
// 種類を問わず後片付けを待たずに終了する
pub(crate) fn watch() {
    watch_platform().expect("Failed to set signal handler");
}

// ターミナルの Ctrl+C は ffmpeg にも届き、ffmpeg はすぐに終わることがある。印を別のスレッドで立てると、
// ffmpeg の失敗を受け取って main に戻るまでに間に合わず、中断ではなくエラーとして報告してしまう。
// そのため Unix では、シグナルを受けたその場(シグナルハンドラの中)で印を立てる
#[cfg(unix)]
fn watch_platform() -> std::io::Result<()> {
    use signal_hook::consts::{SIGHUP, SIGINT, SIGTERM};
    use signal_hook::flag;

    // 同じシグナルに登録した処理は、登録順に実行される
    for signal in [SIGINT, SIGTERM, SIGHUP] {
        // 印がすでに立っていれば終了する処理を、印を立てる処理より先に登録する
        flag::register_conditional_shutdown(signal, exit_code_for(signal), &REQUESTED)?;
        // 印を見た側が終了コードを決められるよう、どのシグナルかを印より先に記録する
        flag::register_usize(signal, &SIGNAL, signal as usize)?;
        flag::register(signal, &REQUESTED)?;
    }
    Ok(())
}

#[cfg(not(unix))]
fn watch_platform() -> Result<(), ctrlc::Error> {
    ctrlc::set_handler(|| {
        if REQUESTED.swap(true, Ordering::Relaxed) {
            std::process::exit(exit_code());
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

/// 中断したときの終了コード
pub(crate) fn exit_code() -> i32 {
    exit_code_for(SIGNAL.load(Ordering::Relaxed) as i32)
}

// 128 + シグナル番号。シグナルで中断されたコマンドの慣習的な終了コード(SIGINT なら 130)
fn exit_code_for(signal: i32) -> i32 {
    128 + signal
}
