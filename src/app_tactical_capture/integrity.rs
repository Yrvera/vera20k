//! No-dependency evidence-integrity primitives for tactical capture.
//!
//! This module owns strict duplicate-free JSON parsing, reparse-safe absolute
//! file checks, stable reads, and streaming SHA-256. It has no app or sim
//! authority.

use std::fmt::Write as _;
use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::{Context, Result, bail, ensure};
use serde::de::{DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::{Map, Value};

const MAX_JSON_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Debug, Clone)]
pub(crate) struct SealedJsonFile<T> {
    pub(crate) path: PathBuf,
    pub(crate) byte_length: u64,
    pub(crate) sha256: String,
    pub(crate) bytes: Vec<u8>,
    pub(crate) value: T,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FileDigest {
    pub(crate) byte_length: u64,
    pub(crate) sha256: String,
}

pub(crate) fn validate_new_output_directory(path: &Path) -> Result<()> {
    ensure!(path.is_absolute(), "--output must be an absolute path");
    reject_reparse_ancestors(path, "tactical output")?;
    match fs::symlink_metadata(path) {
        Ok(_) => bail!(
            "--output already exists; tactical bundles are immutable: {}",
            path.display()
        ),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(error).with_context(|| format!("cannot inspect output {}", path.display()));
        }
    }
    let parent = path
        .parent()
        .context("--output must have a parent directory")?;
    reject_reparse_ancestors(parent, "tactical output parent")?;
    let metadata = fs::metadata(parent)
        .with_context(|| format!("cannot inspect output parent {}", parent.display()))?;
    ensure!(
        metadata.is_dir(),
        "tactical output parent is not a directory: {}",
        parent.display()
    );
    Ok(())
}

pub(crate) fn require_absolute_regular_non_reparse(path: &Path, label: &str) -> Result<()> {
    ensure!(path.is_absolute(), "{label} must be an absolute path");
    reject_reparse_ancestors(path, label)?;
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("{label} does not exist: {}", path.display()))?;
    ensure!(
        metadata.file_type().is_file() && !metadata.file_type().is_symlink(),
        "{label} must be a regular non-link file: {}",
        path.display()
    );
    Ok(())
}

fn reject_reparse_ancestors(path: &Path, label: &str) -> Result<()> {
    for component in path.ancestors() {
        let metadata = match fs::symlink_metadata(component) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(error).with_context(|| {
                    format!("cannot inspect {label} ancestor {}", component.display())
                });
            }
        };
        ensure!(
            !metadata.file_type().is_symlink() && !metadata_is_reparse(&metadata),
            "{label} crosses a link, junction, or reparse point: {}",
            component.display()
        );
    }
    Ok(())
}

#[cfg(windows)]
fn metadata_is_reparse(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn metadata_is_reparse(_metadata: &fs::Metadata) -> bool {
    false
}

fn metadata_stamp(metadata: &fs::Metadata) -> (u64, Option<SystemTime>) {
    (metadata.len(), metadata.modified().ok())
}

pub(crate) fn read_stable_regular_bytes(path: &Path, label: &str) -> Result<(Vec<u8>, FileDigest)> {
    require_absolute_regular_non_reparse(path, label)?;
    let before = fs::symlink_metadata(path)?;
    ensure!(
        before.len() <= MAX_JSON_BYTES,
        "{label} exceeds JSON size limit"
    );
    let bytes =
        fs::read(path).with_context(|| format!("cannot read {label} {}", path.display()))?;
    let after = fs::symlink_metadata(path)?;
    ensure!(
        metadata_stamp(&before) == metadata_stamp(&after) && bytes.len() as u64 == before.len(),
        "{label} changed while being read: {}",
        path.display()
    );
    let digest = FileDigest {
        byte_length: bytes.len() as u64,
        sha256: sha256_hex(&bytes),
    };
    Ok((bytes, digest))
}

pub(crate) fn sha256_file(path: &Path, label: &str) -> Result<FileDigest> {
    require_absolute_regular_non_reparse(path, label)?;
    let before = fs::symlink_metadata(path)?;
    let file =
        File::open(path).with_context(|| format!("cannot open {label} {}", path.display()))?;
    let handle_before = file.metadata()?;
    let mut reader = BufReader::new(file);
    let mut sha = Sha256::new();
    let mut byte_length = 0_u64;
    let mut buffer = vec![0_u8; 1024 * 1024];
    loop {
        let count = reader
            .read(&mut buffer)
            .with_context(|| format!("cannot hash {label} {}", path.display()))?;
        if count == 0 {
            break;
        }
        sha.update(&buffer[..count]);
        byte_length = byte_length
            .checked_add(count as u64)
            .context("file length overflow while hashing")?;
    }
    let handle_after = reader.get_ref().metadata()?;
    let after = fs::symlink_metadata(path)?;
    ensure!(
        metadata_stamp(&before) == metadata_stamp(&handle_before)
            && metadata_stamp(&before) == metadata_stamp(&handle_after)
            && metadata_stamp(&before) == metadata_stamp(&after)
            && byte_length == before.len(),
        "{label} changed while being hashed: {}",
        path.display()
    );
    Ok(FileDigest {
        byte_length,
        sha256: digest_hex(sha.finalize()),
    })
}

pub(crate) fn sha256_bytes(bytes: &[u8]) -> [u8; 32] {
    let mut sha = Sha256::new();
    sha.update(bytes);
    sha.finalize()
}

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    digest_hex(sha256_bytes(bytes))
}

fn digest_hex(digest: [u8; 32]) -> String {
    let mut output = String::with_capacity(64);
    for byte in digest {
        write!(&mut output, "{byte:02x}").expect("writing into String cannot fail");
    }
    output
}

pub(crate) fn parse_strict_json<T>(bytes: &[u8], label: &str) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let strict = StrictJson::deserialize(&mut deserializer)
        .with_context(|| format!("{label} is not strict JSON"))?;
    deserializer
        .end()
        .with_context(|| format!("{label} has trailing data"))?;
    serde_json::from_value(strict.0)
        .with_context(|| format!("{label} has invalid types, values, or keys"))
}

struct StrictJson(Value);

impl<'de> Deserialize<'de> for StrictJson {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(StrictJsonVisitor)
    }
}

struct StrictJsonSeed;

impl<'de> DeserializeSeed<'de> for StrictJsonSeed {
    type Value = StrictJson;

    fn deserialize<D>(self, deserializer: D) -> std::result::Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        StrictJson::deserialize(deserializer)
    }
}

struct StrictJsonVisitor;

impl<'de> Visitor<'de> for StrictJsonVisitor {
    type Value = StrictJson;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("strict finite JSON")
    }

    fn visit_bool<E>(self, value: bool) -> std::result::Result<Self::Value, E> {
        Ok(StrictJson(Value::Bool(value)))
    }

    fn visit_i64<E>(self, value: i64) -> std::result::Result<Self::Value, E> {
        Ok(StrictJson(Value::Number(value.into())))
    }

    fn visit_u64<E>(self, value: u64) -> std::result::Result<Self::Value, E> {
        Ok(StrictJson(Value::Number(value.into())))
    }

    fn visit_f64<E>(self, value: f64) -> std::result::Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        if !value.is_finite() {
            return Err(E::custom("non-finite JSON number"));
        }
        serde_json::Number::from_f64(value)
            .map(Value::Number)
            .map(StrictJson)
            .ok_or_else(|| E::custom("non-finite JSON number"))
    }

    fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E> {
        Ok(StrictJson(Value::String(value.to_owned())))
    }

    fn visit_string<E>(self, value: String) -> std::result::Result<Self::Value, E> {
        Ok(StrictJson(Value::String(value)))
    }

    fn visit_none<E>(self) -> std::result::Result<Self::Value, E> {
        Ok(StrictJson(Value::Null))
    }

    fn visit_unit<E>(self) -> std::result::Result<Self::Value, E> {
        Ok(StrictJson(Value::Null))
    }

    fn visit_some<D>(self, deserializer: D) -> std::result::Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        StrictJson::deserialize(deserializer)
    }

    fn visit_seq<A>(self, mut sequence: A) -> std::result::Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::new();
        while let Some(value) = sequence.next_element_seed(StrictJsonSeed)? {
            values.push(value.0);
        }
        Ok(StrictJson(Value::Array(values)))
    }

    fn visit_map<A>(self, mut map: A) -> std::result::Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut values = Map::new();
        while let Some(key) = map.next_key::<String>()? {
            if values.contains_key(&key) {
                return Err(serde::de::Error::custom(format!(
                    "duplicate JSON object key {key:?}"
                )));
            }
            let value = map.next_value_seed(StrictJsonSeed)?;
            values.insert(key, value.0);
        }
        Ok(StrictJson(Value::Object(values)))
    }
}

struct Sha256 {
    state: [u32; 8],
    block: [u8; 64],
    block_len: usize,
    total_len: u64,
}

impl Sha256 {
    fn new() -> Self {
        Self {
            state: [
                0x6a09_e667,
                0xbb67_ae85,
                0x3c6e_f372,
                0xa54f_f53a,
                0x510e_527f,
                0x9b05_688c,
                0x1f83_d9ab,
                0x5be0_cd19,
            ],
            block: [0; 64],
            block_len: 0,
            total_len: 0,
        }
    }

    fn update(&mut self, mut bytes: &[u8]) {
        self.total_len = self
            .total_len
            .checked_add(bytes.len() as u64)
            .expect("SHA-256 input length overflow");
        if self.block_len != 0 {
            let count = (64 - self.block_len).min(bytes.len());
            self.block[self.block_len..self.block_len + count].copy_from_slice(&bytes[..count]);
            self.block_len += count;
            bytes = &bytes[count..];
            if self.block_len < 64 {
                return;
            }
            let block = self.block;
            self.compress(&block);
            self.block_len = 0;
        }
        while bytes.len() >= 64 {
            let (block, remaining) = bytes.split_at(64);
            let mut owned = [0_u8; 64];
            owned.copy_from_slice(block);
            self.compress(&owned);
            bytes = remaining;
        }
        self.block[..bytes.len()].copy_from_slice(bytes);
        self.block_len = bytes.len();
    }

    fn finalize(mut self) -> [u8; 32] {
        let bit_length = self
            .total_len
            .checked_mul(8)
            .expect("SHA-256 bit length overflow");
        self.block[self.block_len] = 0x80;
        self.block_len += 1;
        if self.block_len > 56 {
            self.block[self.block_len..].fill(0);
            let block = self.block;
            self.compress(&block);
            self.block = [0; 64];
        } else {
            self.block[self.block_len..56].fill(0);
        }
        self.block[56..64].copy_from_slice(&bit_length.to_be_bytes());
        let block = self.block;
        self.compress(&block);
        let mut output = [0_u8; 32];
        for (chunk, word) in output.chunks_exact_mut(4).zip(self.state) {
            chunk.copy_from_slice(&word.to_be_bytes());
        }
        output
    }

    fn compress(&mut self, block: &[u8; 64]) {
        const K: [u32; 64] = [
            0x428a_2f98,
            0x7137_4491,
            0xb5c0_fbcf,
            0xe9b5_dba5,
            0x3956_c25b,
            0x59f1_11f1,
            0x923f_82a4,
            0xab1c_5ed5,
            0xd807_aa98,
            0x1283_5b01,
            0x2431_85be,
            0x550c_7dc3,
            0x72be_5d74,
            0x80de_b1fe,
            0x9bdc_06a7,
            0xc19b_f174,
            0xe49b_69c1,
            0xefbe_4786,
            0x0fc1_9dc6,
            0x240c_a1cc,
            0x2de9_2c6f,
            0x4a74_84aa,
            0x5cb0_a9dc,
            0x76f9_88da,
            0x983e_5152,
            0xa831_c66d,
            0xb003_27c8,
            0xbf59_7fc7,
            0xc6e0_0bf3,
            0xd5a7_9147,
            0x06ca_6351,
            0x1429_2967,
            0x27b7_0a85,
            0x2e1b_2138,
            0x4d2c_6dfc,
            0x5338_0d13,
            0x650a_7354,
            0x766a_0abb,
            0x81c2_c92e,
            0x9272_2c85,
            0xa2bf_e8a1,
            0xa81a_664b,
            0xc24b_8b70,
            0xc76c_51a3,
            0xd192_e819,
            0xd699_0624,
            0xf40e_3585,
            0x106a_a070,
            0x19a4_c116,
            0x1e37_6c08,
            0x2748_774c,
            0x34b0_bcb5,
            0x391c_0cb3,
            0x4ed8_aa4a,
            0x5b9c_ca4f,
            0x682e_6ff3,
            0x748f_82ee,
            0x78a5_636f,
            0x84c8_7814,
            0x8cc7_0208,
            0x90be_fffa,
            0xa450_6ceb,
            0xbef9_a3f7,
            0xc671_78f2,
        ];
        let mut words = [0_u32; 64];
        for (index, chunk) in block.chunks_exact(4).enumerate() {
            words[index] = u32::from_be_bytes(chunk.try_into().expect("four-byte chunk"));
        }
        for index in 16..64 {
            let s0 = words[index - 15].rotate_right(7)
                ^ words[index - 15].rotate_right(18)
                ^ (words[index - 15] >> 3);
            let s1 = words[index - 2].rotate_right(17)
                ^ words[index - 2].rotate_right(19)
                ^ (words[index - 2] >> 10);
            words[index] = words[index - 16]
                .wrapping_add(s0)
                .wrapping_add(words[index - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = self.state;
        for index in 0..64 {
            let sum1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let choice = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(sum1)
                .wrapping_add(choice)
                .wrapping_add(K[index])
                .wrapping_add(words[index]);
            let sum0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let majority = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = sum0.wrapping_add(majority);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }
        for (slot, value) in self.state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *slot = slot.wrapping_add(value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_matches_known_vectors() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );

        let bytes = b"chunk boundaries must not change a streaming SHA-256 digest";
        let mut chunked = Sha256::new();
        for chunk in bytes.chunks(7) {
            chunked.update(chunk);
        }
        assert_eq!(digest_hex(chunked.finalize()), sha256_hex(bytes));
    }
}
