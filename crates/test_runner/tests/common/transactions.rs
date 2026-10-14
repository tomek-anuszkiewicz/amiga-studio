//! SingleStepTests bus transaction deserialization and cycle-exact validation
//!
//! Compares recorded M68000 CPU bus transactions against silicon reference logs
//! from Tom Harte SingleStepTests-680x0 physical hardware captures.

use serde::{Deserialize, Serialize};

/// Bus access transfer size for test-runner cycle validation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BusAccessSize {
    Byte,
    Word,
}

/// Simplified recorded CPU bus cycle transaction for test validation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordedTransaction {
    /// Observed CPU clock at the memory callback: reads in CCK1, writes in CCK2.
    pub clock: u32,
    pub is_read: bool,
    pub addr: u32,
    pub size: BusAccessSize,
    pub data: u16,
}

/// Expected transaction parsed from SingleStepTests JSON
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpectedTransaction {
    Bus {
        is_read: bool,
        duration: u32,
        addr: u32,
        size: BusAccessSize,
        data: u16,
    },
    Internal {
        duration: u32,
    },
}

/// Parses raw JSON array of transactions from a SingleStepTest
pub fn parse_transactions(
    values: &[serde_json::Value],
    initial_ram: &[[u32; 2]],
) -> Result<Vec<ExpectedTransaction>, String> {
    let mut transactions = Vec::with_capacity(values.len());
    for (idx, val) in values.iter().enumerate() {
        let arr = val
            .as_array()
            .ok_or_else(|| format!("Transaction [{}] is not a JSON array: {:?}", idx, val))?;
        if arr.is_empty() {
            return Err(format!("Transaction [{}] is an empty array", idx));
        }
        let tag = arr[0]
            .as_str()
            .ok_or_else(|| format!("Transaction [{}] tag is not a string: {:?}", idx, arr[0]))?;

        if tag == "n" {
            if arr.len() != 2 {
                return Err(format!("Internal transaction [{idx}] requires two fields"));
            }
            let duration = transaction_number(arr, 1, u32::MAX as u64, idx)? as u32;
            transactions.push(ExpectedTransaction::Internal { duration });
        } else {
            if !matches!(tag, "r" | "w" | "t") {
                return Err(format!("Unknown transaction tag '{tag}' at [{idx}]"));
            }
            if arr.len() != 6 {
                return Err(format!("Bus transaction [{idx}] requires six fields"));
            }
            let is_read = tag == "r";
            let is_tas = tag == "t";
            let duration = transaction_number(arr, 1, u32::MAX as u64, idx)? as u32;
            // Width validation is not observation of an electrical FC signal.
            let _fc = transaction_number(arr, 2, 7, idx)? as u8;
            let addr = transaction_number(arr, 3, 0xFFFFFF, idx)? as u32;
            let size_str = arr
                .get(4)
                .and_then(|v| v.as_str())
                .ok_or_else(|| format!("Bus transaction [{}] missing size string", idx))?;
            let size = match size_str {
                ".b" => BusAccessSize::Byte,
                ".w" => BusAccessSize::Word,
                other => return Err(format!("Unknown transfer size '{}' at [{}]", other, idx)),
            };
            let max_data = if size == BusAccessSize::Byte {
                255
            } else {
                65535
            };
            let data = transaction_number(arr, 5, max_data, idx)? as u16;

            if is_tas {
                // Motorola Section 5.1.3: S0..S7 read, S8..S11 modify,
                // S12..S19 write. JSON reports the final byte for this interval.
                if duration != 10 || size != BusAccessSize::Byte {
                    return Err(format!("Unsupported TAS interval at [{}]", idx));
                }
                let original = initial_ram
                    .iter()
                    .rev()
                    .find(|entry| (entry[0] & 0x00ff_ffff) == addr)
                    .map(|entry| entry[1] as u16)
                    .ok_or_else(|| {
                        format!("TAS read byte missing from initial RAM at [{}]", idx)
                    })?;
                transactions.push(ExpectedTransaction::Bus {
                    is_read: true,
                    duration: 4,
                    addr,
                    size,
                    data: original,
                });
                transactions.push(ExpectedTransaction::Internal { duration: 2 });
                transactions.push(ExpectedTransaction::Bus {
                    is_read: false,
                    duration: 4,
                    addr,
                    size,
                    data,
                });
                continue;
            }
            transactions.push(ExpectedTransaction::Bus {
                is_read,
                duration,
                addr,
                size,
                data,
            });
        }
    }
    Ok(transactions)
}

fn transaction_number(
    arr: &[serde_json::Value],
    field: usize,
    max: u64,
    index: usize,
) -> Result<u64, String> {
    arr.get(field)
        .and_then(|value| value.as_u64())
        .filter(|&value| value <= max)
        .ok_or_else(|| {
            format!("Transaction [{index}] field {field} requires an integer in 0..={max}")
        })
}

/// Matches recorded CPU bus transactions against the expected test sequence
pub fn match_transactions(
    recorded: &[RecordedTransaction],
    expected: &[ExpectedTransaction],
) -> Result<(), Vec<String>> {
    let mut diffs = Vec::new();

    // Internal events advance the reference timeline even though they have no bus callback.
    let mut clock = 0u32;
    let mut expected_bus = Vec::new();
    for transaction in expected {
        match transaction {
            ExpectedTransaction::Internal { duration } => clock = clock.wrapping_add(*duration),
            ExpectedTransaction::Bus {
                is_read, duration, ..
            } => {
                // The core calls reads at the end of CCK1 and commits writes at CCK2.
                // Compare that observed phase to the reference bus interval.
                let callback_clock = clock.wrapping_add(if *is_read { 2 } else { *duration });
                expected_bus.push((callback_clock, transaction));
                clock = clock.wrapping_add(*duration);
            }
        }
    }

    if recorded.len() != expected_bus.len() {
        diffs.push(format!(
            "Transaction count mismatch: recorded {} bus transactions, expected {}",
            recorded.len(),
            expected_bus.len()
        ));
    }

    let compare_count = recorded.len().min(expected_bus.len());
    for i in 0..compare_count {
        let rec = &recorded[i];
        if let ExpectedTransaction::Bus {
            is_read: exp_r,
            duration: exp_dur,
            addr: exp_addr,
            size: exp_size,
            data: exp_data,
            ..
        } = expected_bus[i].1
        {
            if rec.clock != expected_bus[i].0 {
                diffs.push(format!(
                    "Transaction [{}]: Callback clock mismatch: actual {}, expected {} CPU clocks",
                    i, rec.clock, expected_bus[i].0
                ));
            }
            let exp_read = *exp_r;
            if rec.is_read != exp_read {
                diffs.push(format!(
                    "Transaction [{}]: Direction mismatch: actual read={}, expected read={}",
                    i, rec.is_read, exp_read
                ));
            }
            if rec.addr != *exp_addr {
                diffs.push(format!(
                    "Transaction [{}]: Address mismatch: actual ${:06X}, expected ${:06X}",
                    i, rec.addr, exp_addr
                ));
            }
            if rec.size != *exp_size {
                diffs.push(format!(
                    "Transaction [{}]: Access size mismatch: actual {:?}, expected {:?}",
                    i, rec.size, exp_size
                ));
            }
            if *exp_dur != 4 {
                diffs.push(format!(
                    "Transaction [{}]: Duration mismatch: actual 4 clocks, expected {} clocks",
                    i, exp_dur
                ));
            }
            // Data comparison
            match rec.size {
                BusAccessSize::Word => {
                    if rec.data != *exp_data {
                        diffs.push(format!(
                            "Transaction [{}]: Word data mismatch: actual ${:04X}, expected ${:04X}",
                            i, rec.data, exp_data
                        ));
                    }
                }
                BusAccessSize::Byte => {
                    let actual_byte = (rec.data & 0xFF) as u8;
                    let expected_byte = (*exp_data & 0xFF) as u8;
                    if actual_byte != expected_byte {
                        diffs.push(format!(
                            "Transaction [{}]: Byte data mismatch: actual ${:02X}, expected ${:02X}",
                            i, actual_byte, expected_byte
                        ));
                    }
                }
            }
        }
    }

    if diffs.is_empty() {
        Ok(())
    } else {
        Err(diffs)
    }
}
