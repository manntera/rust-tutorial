// 新しい機能別フォルダ構成
pub mod app;
pub mod benchmarks;
pub mod cli;
pub mod core;
pub mod engine;
pub mod factories;
pub mod services;

// 従来のモジュール
pub mod image_loader;
pub mod perceptual_hash;
pub mod storage;

// 下位互換性のため、従来のAPIを再エクスポート
pub use app::App;
// core モジュールから明示的にエクスポート
pub use core::{
    ProcessingError, ProcessingResult,
    HashPersistence, ParallelProcessor, ProcessingConfig, ProgressReporter,
    ProcessingOutcome, ProcessingMetadata, ProcessingSummary,
};
// engine モジュールから明示的にエクスポート  
pub use engine::{
    ProcessingEngine,
    process_directory_with_engine, process_files_with_engine,
    create_default_processing_engine, create_quiet_processing_engine,
};
// services モジュールから明示的にエクスポート
pub use services::{
    DefaultProcessingConfig,
    ConsoleProgressReporter, NoOpProgressReporter,
    JsonHashPersistence, MemoryHashPersistence, StreamingJsonHashPersistence,
    spawn_result_collector,
    process_single_file,
};
