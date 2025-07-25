use std::env;
use std::process::Command;

fn main() {
    let commands = [
        ("cargo test", "テスト実行"),
        ("cargo clippy -- -D warnings", "Clippy実行"),
        ("cargo tarpaulin --out Html", "テストカバレッジ測定"),
    ];

    let current_dir = match env::current_dir() {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("現在のディレクトリを取得できません: {e}");
            std::process::exit(1);
        }
    };

    for (cmd, desc) in &commands {
        println!("=== {desc} ===");
        let output = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(["/C", cmd])
                .current_dir(&current_dir)
                .output()
        } else {
            Command::new("sh")
                .arg("-c")
                .arg(cmd)
                .current_dir(&current_dir)
                .output()
        };

        match output {
            Ok(output) => {
                if !output.status.success() {
                    eprintln!(
                        "Error executing {cmd}: {}",
                        String::from_utf8_lossy(&output.stderr)
                    );
                    std::process::exit(1);
                }
                println!("{}", String::from_utf8_lossy(&output.stdout));
            }
            Err(e) => {
                eprintln!("Failed to execute {cmd}: {e}");
                std::process::exit(1);
            }
        }
    }

    println!("すべてのチェックが完了しました！");
}
