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
        is_tas: bool,
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
            let duration =
                arr.get(1).and_then(|v| v.as_u64()).ok_or_else(|| {
                    format!("Internal transaction [{}] missing numeric duration", idx)
                })? as u32;
            transactions.push(ExpectedTransaction::Internal { duration });
        } else {
            let is_read = tag == "r";
            let is_tas = tag == "t";
            let duration = arr
                .get(1)
                .and_then(|v| v.as_u64())
                .ok_or_else(|| format!("Bus transaction [{}] missing duration", idx))?
                as u32;
            let _fc = arr
                .get(2)
                .and_then(|v| v.as_u64())
                .ok_or_else(|| format!("Bus transaction [{}] missing function code", idx))?
                as u8;
            let addr = arr
                .get(3)
                .and_then(|v| v.as_u64())
                .ok_or_else(|| format!("Bus transaction [{}] missing address", idx))?
                as u32;
            let size_str = arr
                .get(4)
                .and_then(|v| v.as_str())
                .ok_or_else(|| format!("Bus transaction [{}] missing size string", idx))?;
            let size = match size_str {
                ".b" => BusAccessSize::Byte,
                ".w" => BusAccessSize::Word,
                other => return Err(format!("Unknown transfer size '{}' at [{}]", other, idx)),
            };
            let data = arr
                .get(5)
                .and_then(|v| v.as_u64())
                .ok_or_else(|| format!("Bus transaction [{}] missing data value", idx))?
                as u16;

            transactions.push(ExpectedTransaction::Bus {
                is_read,
                is_tas,
                duration,
                addr,
                size,
                data,
            });
        }
    }
    Ok(transactions)
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
                is_read,
                is_tas,
                duration,
                ..
            } => {
                // The core calls reads at the end of CCK1 and commits writes at CCK2.
                // Compare that observed phase to the reference bus interval.
                let callback_clock =
                    clock.wrapping_add(if *is_read || *is_tas { 2 } else { *duration });
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
            is_tas: exp_tas,
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
            let exp_read = *exp_r || *exp_tas;
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
