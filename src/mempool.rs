use bitcoin::consensus::encode::Decodable;
use bitcoin::io as bitcoin_io;
use bitcoin::transaction::{Transaction, Txid};
use byteorder::{LittleEndian, ReadBytesExt};
use std::fmt;
use std::fs::File;
use std::io::{self, BufReader, Read, Seek};
use std::path::Path;
use thiserror::Error;

use crate::stream::XorReader;

const MEMPOOL_V2_FORMAT: u64 = 2; // Requires an XOR key to be read from .dat

#[derive(Error, Debug)]
pub enum MempoolError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Failed to read header: {0}")]
    HeaderRead(String),

    #[error("Failed to read mempool entry at index {0}: {1}")]
    EntryRead(usize, String),

    #[error("Failed to read XOR key: {0}")]
    XorKeyRead(String),
}

#[derive(Debug, Clone, Copy)]
pub struct FileHeader {
    pub version: u64,
    pub num_tx: u64,
}

impl FileHeader {
    pub fn new(version: u64, num_tx: u64) -> Self {
        Self { version, num_tx }
    }
}

impl fmt::Display for FileHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Version {}, {} transactions", self.version, self.num_tx)
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct MempoolEntry {
    pub first_seen_time: i64,
    pub fee_delta: i64,
    pub transaction: Transaction,
}

impl MempoolEntry {
    pub fn new(transaction: Transaction, first_seen_time: i64, fee_delta: i64) -> Self {
        Self {
            transaction,
            first_seen_time,
            fee_delta,
        }
    }
}

impl fmt::Display for MempoolEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "{:#?}", self)
        } else {
            write!(f, "{:?}", self)
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct FeeDelta {
    pub txid: Txid,
    pub delta: i64,
}

// A parsed mempool.dat
#[allow(dead_code)]
#[derive(Debug)]
pub struct Mempool {
    pub header: FileHeader,
    pub xor_key: Option<Vec<u8>>,
    pub entries: Vec<MempoolEntry>,
    pub map_deltas: Vec<FeeDelta>,
}

impl Mempool {
    pub fn new(
        header: FileHeader,
        entries: Vec<MempoolEntry>,
        map_deltas: Vec<FeeDelta>,
        xor_key: Option<Vec<u8>>,
    ) -> Self {
        Self {
            header,
            entries,
            map_deltas,
            xor_key,
        }
    }

    pub fn get_mempool_entries(&self) -> &[MempoolEntry] {
        &self.entries
    }

    pub fn get_file_header(&self) -> &FileHeader {
        &self.header
    }

    pub fn get_xor_key(&self) -> Option<&[u8]> {
        self.xor_key.as_deref()
    }
}

pub fn read_mempool_from_path<P: AsRef<Path>>(path: P) -> Result<Mempool, MempoolError> {
    let file = File::open(&path)?;
    let mut reader = BufReader::new(file);

    // version is never xored
    let version = reader
        .read_u64::<LittleEndian>()
        .map_err(|e| MempoolError::HeaderRead(format!("Failed to read version: {}", e)))?;

    let xor_key = if version == MEMPOOL_V2_FORMAT {
        let mut size_buf = [0u8; 1];
        reader
            .read_exact(&mut size_buf)
            .map_err(|e| MempoolError::XorKeyRead(format!("Failed to read XOR key size: {}", e)))?;
        let key_size = size_buf[0] as usize;
        let mut key = vec![0u8; key_size];
        reader.read_exact(&mut key).map_err(|e| {
            MempoolError::XorKeyRead(format!("Failed to read XOR key from mempool file: {}", e))
        })?;
        Some(key)
    } else {
        None
    };

    let mut xor_reader = XorReader::new(reader, xor_key.clone().unwrap_or_default())?;

    // For V2 format, we need to start XOR from the transaction count
    // The num_tx value needs to be decrypted using the XOR key
    let num_tx = xor_reader
        .read_u64_le()
        .map_err(|e| MempoolError::HeaderRead(format!("Failed to read tx count: {}", e)))?;

    let header = FileHeader::new(version, num_tx);
    let mut entries = Vec::with_capacity(num_tx as usize);
    for i in 0..num_tx {
        let entry = read_mempool_entry(&mut xor_reader)
            .map_err(|e| MempoolError::EntryRead(i as usize, e.to_string()))?;
        entries.push(entry);
    }

    // TODO: implement mapDeltas
    let map_deltas = Vec::new();

    Ok(Mempool::new(header, entries, map_deltas, xor_key))
}

// Read a mempool entry
// Use rust-bitcoin to deserialize the transaction
fn read_mempool_entry<R: Read + Seek>(
    reader: &mut XorReader<R>,
) -> Result<MempoolEntry, io::Error> {
    struct BitcoinReader<'a, R: Read + Seek>(&'a mut XorReader<R>);

    impl<R: Read + Seek> bitcoin_io::Read for BitcoinReader<'_, R> {
        fn read(&mut self, buf: &mut [u8]) -> Result<usize, bitcoin_io::Error> {
            self.0.read(buf).map_err(|e| e.into())
        }
    }

    let mut bitcoin_reader = BitcoinReader(reader);
    let transaction = Transaction::consensus_decode(&mut bitcoin_reader).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to decode transaction: {}", e),
        )
    })?;
    let timestamp = reader.read_i64_le()?;
    let fee_delta = reader.read_i64_le()?;

    Ok(MempoolEntry::new(transaction, timestamp, fee_delta))
}
