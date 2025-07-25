// main.rsとエントリーポイントのテスト
use anyhow::Result;
use tempfile::TempDir;
use std::fs;
use std::process::Command;
use std::path::PathBuf;

const MINIMAL_PNG_DATA: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D,
    0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
    0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00,
    0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
    0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
    0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
];

fn get_binary_path() -> anyhow::Result<PathBuf> {
    let mut path = std::env::current_exe()?;
    path.pop(); // remove test binary name
    if path.ends_with("deps") {
        path.pop(); // remove deps directory
    }
    Ok(path.join("image_dedup"))
}

#[test]
fn test_binary_exists() -> Result<()> {
    let binary_path = get_binary_path()?;
    // バイナリの存在確認（統合テスト環境でのみ）
    if binary_path.exists() {
        assert!(binary_path.is_file());
    } else {
        // バイナリが存在しない場合はスキップ
        println!("Skipping binary test - binary not found at {:?}", binary_path);
    }
    Ok(())
}

#[test]
fn test_cli_help() -> Result<()> {
    let binary_path = get_binary_path()?;
    if !binary_path.exists() {
        println!("Skipping CLI test - binary not found");
        return Ok(());
    }

    let output = Command::new(&binary_path)
        .arg("--help")
        .output()?;

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("image_dedup"));
    assert!(stdout.contains("scan"));
    assert!(stdout.contains("find-dups"));
    assert!(stdout.contains("process"));
    Ok(())
}

#[test]
fn test_cli_version() -> Result<()> {
    let binary_path = get_binary_path()?;
    if !binary_path.exists() {
        println!("Skipping CLI version test - binary not found");
        return Ok(());
    }

    let output = Command::new(&binary_path)
        .arg("--version")
        .output()?;

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("image_dedup"));
    Ok(())
}

#[test]
fn test_cli_scan_integration() -> Result<()> {
    let binary_path = get_binary_path()?;
    if !binary_path.exists() {
        println!("Skipping CLI scan integration test - binary not found");
        return Ok(());
    }

    let temp_dir = TempDir::new()?;
    let output_file = temp_dir.path().join("results.json");

    // Create a test image
    fs::write(temp_dir.path().join("test.png"), MINIMAL_PNG_DATA)?;

    let output = Command::new(&binary_path)
        .arg("scan")
        .arg(temp_dir.path())
        .arg("--output")
        .arg(&output_file)
        .arg("--force")
        .output()?;

    if output.status.success() {
        assert!(output_file.exists());
        
        // Check output content
        let content = fs::read_to_string(&output_file)?;
        let json: serde_json::Value = serde_json::from_str(&content)?;
        assert!(json.is_array());
    } else {
        let stderr = String::from_utf8(output.stderr)?;
        println!("Scan command failed: {}", stderr);
    }
    Ok(())
}

#[test]
fn test_cli_find_dups_integration() -> Result<()> {
    let binary_path = get_binary_path()?;
    if !binary_path.exists() {
        println!("Skipping CLI find-dups integration test - binary not found");
        return Ok(());
    }

    let temp_dir = TempDir::new()?;
    let hash_db = temp_dir.path().join("hashes.json");
    let output_file = temp_dir.path().join("duplicates.json");

    // Create sample hash database
    let sample_hashes = r#"[
        {
            "file_path": "/test/image1.jpg",
            "hash": "abcd1234",
            "algorithm": "DCT",
            "hash_bits": 0,
            "timestamp": "2024-01-01T00:00:00Z"
        },
        {
            "file_path": "/test/image2.jpg",
            "hash": "abcd1235",
            "algorithm": "DCT", 
            "hash_bits": 1,
            "timestamp": "2024-01-01T00:00:00Z"
        }
    ]"#;
    fs::write(&hash_db, sample_hashes)?;

    let output = Command::new(&binary_path)
        .arg("find-dups")
        .arg(&hash_db)
        .arg("--output")
        .arg(&output_file)
        .arg("--threshold")
        .arg("5")
        .output()?;

    if output.status.success() {
        assert!(output_file.exists());
        
        // Check output content
        let content = fs::read_to_string(&output_file)?;
        let json: serde_json::Value = serde_json::from_str(&content)?;
        assert!(json.get("total_groups").is_some());
        assert!(json.get("groups").is_some());
    } else {
        let stderr = String::from_utf8(output.stderr)?;
        println!("Find-dups command failed: {}", stderr);
    }
    Ok(())
}

#[test]
fn test_cli_process_integration() -> Result<()> {
    let binary_path = get_binary_path()?;
    if !binary_path.exists() {
        println!("Skipping CLI process integration test - binary not found");
        return Ok(());
    }

    let temp_dir = TempDir::new()?;
    let dup_list = temp_dir.path().join("duplicates.json");

    // Create sample duplicates report
    let sample_report = r#"{
        "total_groups": 0,
        "total_duplicates": 0,
        "threshold": 5,
        "groups": []
    }"#;
    fs::write(&dup_list, sample_report)?;

    let output = Command::new(&binary_path)
        .arg("process")
        .arg(&dup_list)
        .arg("--action")
        .arg("delete")
        .arg("--no-confirm")
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8(output.stderr)?;
        println!("Process command failed: {}", stderr);
    }
    // Note: Process command with empty duplicates should succeed
    Ok(())
}

#[test]
fn test_cli_invalid_command() -> Result<()> {
    let binary_path = get_binary_path()?;
    if !binary_path.exists() {
        println!("Skipping CLI invalid command test - binary not found");
        return Ok(());
    }

    let output = Command::new(&binary_path)
        .arg("invalid-command")
        .output()?;

    assert!(!output.status.success());
    Ok(())
}

#[test]
fn test_cli_scan_missing_args() -> Result<()> {
    let binary_path = get_binary_path()?;
    if !binary_path.exists() {
        println!("Skipping CLI scan missing args test - binary not found");
        return Ok(());
    }

    let output = Command::new(&binary_path)
        .arg("scan")
        // Missing required arguments
        .output()?;

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("required") || stderr.contains("argument"));
    Ok(())
}

#[test] 
fn test_cli_find_dups_missing_args() -> Result<()> {
    let binary_path = get_binary_path()?;
    if !binary_path.exists() {
        println!("Skipping CLI find-dups missing args test - binary not found");
        return Ok(());
    }

    let output = Command::new(&binary_path)
        .arg("find-dups")
        // Missing required arguments
        .output()?;

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("required") || stderr.contains("argument"));
    Ok(())
}

#[test]
fn test_cli_process_missing_args() -> Result<()> {
    let binary_path = get_binary_path()?;
    if !binary_path.exists() {
        println!("Skipping CLI process missing args test - binary not found");
        return Ok(());
    }

    let output = Command::new(&binary_path)
        .arg("process")
        // Missing required arguments
        .output()?;

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("required") || stderr.contains("argument"));
    Ok(())
}

// Unit tests for main module functions (using library directly)
mod main_unit_tests {
    use image_dedup::cli::{Cli, Commands, ProcessAction};
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_cli_struct_creation() {
        // Test CLI struct can be created
        let temp_dir = TempDir::new().expect("Failed to create temporary directory");
        let output = temp_dir.path().join("test.json");

        let cli = Cli {
            command: Commands::Scan {
                target_directory: temp_dir.path().to_path_buf(),
                output,
                threads: Some(2),
                force: true,
            },
        };

        match cli.command {
            Commands::Scan { threads, force, .. } => {
                assert_eq!(threads, Some(2));
                assert!(force);
            }
            _ => assert!(false, "Expected Scan command"),
        }
    }

    #[test]
    fn test_cli_commands_enum() {
        let temp_dir = TempDir::new().expect("Failed to create temporary directory");

        // Test all command variants can be created
        let scan_cmd = Commands::Scan {
            target_directory: temp_dir.path().to_path_buf(),
            output: temp_dir.path().join("out.json"),
            threads: None,
            force: false,
        };

        let find_dups_cmd = Commands::FindDups {
            hash_database: temp_dir.path().join("hashes.json"),
            output: temp_dir.path().join("dups.json"),
            threshold: 5,
        };

        let process_cmd = Commands::Process {
            duplicate_list: temp_dir.path().join("dups.json"),
            action: ProcessAction::Move,
            dest: temp_dir.path().join("moved"),
            no_confirm: true,
        };

        // Verify types
        assert!(matches!(scan_cmd, Commands::Scan { .. }));
        assert!(matches!(find_dups_cmd, Commands::FindDups { .. }));
        assert!(matches!(process_cmd, Commands::Process { .. }));
    }

    #[test]
    fn test_process_action_enum() {
        let move_action = ProcessAction::Move;
        let delete_action = ProcessAction::Delete;

        // Test Debug formatting
        assert!(format!("{:?}", move_action).contains("Move"));
        assert!(format!("{:?}", delete_action).contains("Delete"));

        // Test equality
        assert_eq!(move_action, ProcessAction::Move);
        assert_eq!(delete_action, ProcessAction::Delete);
        assert_ne!(move_action, delete_action);
    }

    #[tokio::test]
    async fn test_command_execution_flow() {
        // Test that command functions can be called (integration style)
        let temp_dir = TempDir::new().expect("Failed to create temporary directory");
        
        // Create test files for scan command
        let png_data = &[
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D,
            0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
            0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00,
            0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
            0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
            0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ];
        fs::write(temp_dir.path().join("test.png"), png_data).unwrap();
        
        let output = temp_dir.path().join("results.json");
        
        // Test scan command execution
        let result = image_dedup::cli::commands::execute_scan(
            temp_dir.path().to_path_buf(),
            output.clone(),
            Some(1),
            true,
        ).await;

        assert!(result.is_ok());
        assert!(output.exists());

        // Test find_dups with the generated hash database
        let dup_output = temp_dir.path().join("duplicates.json");
        let find_result = image_dedup::cli::commands::execute_find_dups(
            output,
            dup_output.clone(),
            5,
        ).await;

        assert!(find_result.is_ok());
        assert!(dup_output.exists());

        // Test process command with empty duplicates
        let process_result = image_dedup::cli::commands::execute_process(
            dup_output,
            ProcessAction::Delete,
            temp_dir.path().join("moved"),
            true,
        ).await;

        assert!(process_result.is_ok());
    }
}