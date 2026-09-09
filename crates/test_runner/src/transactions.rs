//! SingleStepTests bus transaction deserialization and cycle-exact validation
//!
//! Compares recorded M68000 CPU bus transactions against silicon reference logs
//! from Tom Harte SingleStepTests-680x0 and MAME SingleStepTests.

use memory_bus::{BusAccessSize, RecordedTransaction};

/// Expected transaction parsed from SingleStepTests JSON
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpectedTransaction {
    Bus {
        is_read: bool,
        is_tas: bool,
        duration: u32,
        fc: u8,
        addr: u32,
        size: BusAccessSize,
        data: u16,
        uds: Option<bool>,
        lds: Option<bool>,
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
            let is_read = tag == "r" || tag == "re";
            let is_tas = tag == "t";
            let duration = arr
                .get(1)
                .and_then(|v| v.as_u64())
                .ok_or_else(|| format!("Bus transaction [{}] missing duration", idx))?
                as u32;
            let fc = arr
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

            let uds = arr.get(6).and_then(|v| v.as_u64()).map(|v| v != 0);
            let lds = arr.get(7).and_then(|v| v.as_u64()).map(|v| v != 0);

            transactions.push(ExpectedTransaction::Bus {
                is_read,
                is_tas,
                duration,
                fc,
                addr: addr & 0x00FF_FFFF,
                size,
                data,
                uds,
                lds,
            });
        }
    }
    Ok(transactions)
}

/// Matches recorded CPU bus transactions against the expected test sequence
pub fn match_transactions(
    recorded: &[RecordedTransaction],
    expected: &[ExpectedTransaction],
    is_harte: bool,
) -> Result<(), Vec<String>> {
    let mut diffs = Vec::new();

    // Filter out internal / non-bus operations ("n") from expected test vectors
    let expected_bus: Vec<&ExpectedTransaction> = expected
        .iter()
        .filter(|tx| matches!(tx, ExpectedTransaction::Bus { .. }))
        .collect();

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
            uds: exp_uds,
            lds: exp_lds,
            ..
        } = expected_bus[i]
        {
            let exp_read = *exp_r || *exp_tas;
            if rec.is_read != exp_read {
                diffs.push(format!(
                    "Transaction [{}]: Direction mismatch: actual read={}, expected read={}",
                    i, rec.is_read, exp_read
                ));
            }
            if (rec.addr & 0x00FF_FFFF) != (exp_addr & 0x00FF_FFFF) {
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
                    let expected_byte = if is_harte {
                        (*exp_data & 0xFF) as u8
                    } else if exp_uds == &Some(true) {
                        ((*exp_data >> 8) & 0xFF) as u8
                    } else {
                        (*exp_data & 0xFF) as u8
                    };
                    if actual_byte != expected_byte {
                        diffs.push(format!(
                            "Transaction [{}]: Byte data mismatch: actual ${:02X}, expected ${:02X}",
                            i, actual_byte, expected_byte
                        ));
                    }
                }
            }
            // Strobe comparison for MAME (if strobes provided)
            if let (Some(exp_u), Some(exp_l)) = (exp_uds, exp_lds) {
                let (rec_uds, rec_lds) = match rec.size {
                    BusAccessSize::Word => (true, true),
                    BusAccessSize::Byte => ((rec.addr & 1) == 0, (rec.addr & 1) != 0),
                };
                if rec_uds != *exp_u || rec_lds != *exp_l {
                    diffs.push(format!(
                        "Transaction [{}]: Strobe mismatch: actual (UDS={}, LDS={}), expected (UDS={}, LDS={})",
                        i, rec_uds, rec_lds, exp_u, exp_l
                    ));
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
