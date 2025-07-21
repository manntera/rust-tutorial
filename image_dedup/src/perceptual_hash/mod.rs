use anyhow::Result;
use async_trait::async_trait;
use image::DynamicImage;
use std::fmt;

pub mod average_hash;
pub mod dct_hash;

/// ハッシュアルゴリズムの種類
#[derive(Debug, Clone, PartialEq)]
pub enum HashAlgorithm {
    /// DCTベースのハッシュ（高精度、計算コスト高）
    DCT { size: u32 },
    /// 平均値ベースのハッシュ（高速、精度は中程度）
    Average { size: u32 },
    /// 差分ベースのハッシュ（エッジ検出に有効）
    Difference { size: u32 },
}

/// ハッシュ計算の結果
#[derive(Debug, Clone)]
pub struct HashResult {
    /// ハッシュ値（バイナリ形式）
    pub hash_data: Vec<u8>,
    /// ハッシュサイズ（ビット数）
    pub hash_size_bits: u32,
    /// 使用されたアルゴリズム
    pub algorithm: HashAlgorithm,
    /// 計算時間（ミリ秒）
    pub computation_time_ms: u64,
    /// 元画像のサイズ
    pub source_dimensions: (u32, u32),
}

impl HashResult {
    /// ハッシュをBase64文字列として取得
    pub fn to_base64(&self) -> String {
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &self.hash_data)
    }

    /// ハッシュを16進文字列として取得
    pub fn to_hex(&self) -> String {
        hex::encode(&self.hash_data)
    }

    /// ハッシュをビット文字列として取得
    pub fn to_bits(&self) -> String {
        self.hash_data
            .iter()
            .map(|byte| format!("{byte:08b}"))
            .collect::<Vec<_>>()
            .join("")
    }
}

impl fmt::Display for HashResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Hash({:?}, {} bits, {}ms): {}",
            self.algorithm,
            self.hash_size_bits,
            self.computation_time_ms,
            self.to_hex()
        )
    }
}

/// 知覚ハッシュバックエンドのトレイト
#[async_trait]
pub trait PerceptualHashBackend: Send + Sync {
    /// 画像からハッシュを生成
    async fn generate_hash(&self, image: &DynamicImage) -> Result<HashResult>;

    /// 2つのハッシュ間の距離を計算（ハミング距離）
    fn calculate_distance(&self, hash1: &HashResult, hash2: &HashResult) -> Result<u32>;

    /// 指定した閾値以下で2つのハッシュが類似しているかを判定
    fn are_similar(&self, hash1: &HashResult, hash2: &HashResult, threshold: u32) -> Result<bool> {
        let distance = self.calculate_distance(hash1, hash2)?;
        Ok(distance <= threshold)
    }

    /// 使用するアルゴリズムを取得
    fn algorithm(&self) -> &HashAlgorithm;

    /// アルゴリズムの名前を取得
    fn algorithm_name(&self) -> &'static str;

    /// 推奨される類似性閾値を取得
    fn recommended_threshold(&self) -> u32 {
        match self.algorithm() {
            HashAlgorithm::DCT { size } => size / 4,
            HashAlgorithm::Average { size } => size / 8,
            HashAlgorithm::Difference { size } => size / 6,
        }
    }

    /// 計算の複雑さを取得（1-10のスケール、10が最も重い）
    fn computational_complexity(&self) -> u8 {
        match self.algorithm() {
            HashAlgorithm::Average { .. } => 2,
            HashAlgorithm::Difference { .. } => 3,
            HashAlgorithm::DCT { .. } => 7,
        }
    }
}

/// ハッシュの比較結果
#[derive(Debug, Clone)]
pub struct ComparisonResult {
    pub distance: u32,
    pub similarity_percentage: f64,
    pub is_similar: bool,
    pub threshold_used: u32,
    pub algorithm: HashAlgorithm,
}

impl ComparisonResult {
    pub fn new(
        distance: u32,
        threshold: u32,
        algorithm: HashAlgorithm,
        hash_size_bits: u32,
    ) -> Self {
        let similarity_percentage = 100.0 * (1.0 - (distance as f64 / hash_size_bits as f64));
        let is_similar = distance <= threshold;

        Self {
            distance,
            similarity_percentage: similarity_percentage.max(0.0),
            is_similar,
            threshold_used: threshold,
            algorithm,
        }
    }
}
