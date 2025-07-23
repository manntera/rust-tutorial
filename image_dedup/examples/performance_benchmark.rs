//! 静的ディスパッチ vs 動的ディスパッチのパフォーマンス比較実行例
//! 
//! 使用方法:
//! ```
//! cargo run --example performance_benchmark
//! ```

use image_dedup::benchmarks::PerformanceComparison;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 画像重複検出ツール - パフォーマンス比較");
    println!("静的ディスパッチ vs 動的ディスパッチの性能測定を開始します...\n");

    // パフォーマンス比較を作成して実行
    let mut comparison = PerformanceComparison::new();
    comparison.run_full_comparison();

    // レポートをJSONで出力
    let report_path = std::path::Path::new("performance_report.json");
    match comparison.export_json_report(report_path) {
        Ok(()) => println!("\n✅ 詳細レポートが {} に出力されました", report_path.display()),
        Err(e) => eprintln!("⚠️  レポート出力エラー: {e}"),
    }

    println!("\n🎯 パフォーマンス比較完了！");
    
    Ok(())
}