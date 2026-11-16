//! Stable execution identities for cold snapshot encoding; no host pointers persist.

use super::{common, MicroStep, OPCODE_DESCRIPTOR_TABLE};
use crate::instructions::{bcc, chk, dbcc, scc, trapv};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Clone, Copy, Serialize, Deserialize)]
enum Sequence {
    Empty,
    Opcode(u16),
    Interrupt,
    Trace,
    AddressError,
    DivideZero,
    Privilege,
    BranchTaken,
    BranchShort,
    BranchWord,
    DbTrue,
    DbTaken,
    DbExpired,
    SccTrue,
    SccFalse,
    ChkNegative,
    ChkUpper,
    TrapOverflow,
}

fn redirected() -> [(Sequence, &'static [MicroStep]); 17] {
    [
        (Sequence::Empty, &[]),
        (Sequence::Interrupt, &common::STEPS_INTERRUPT),
        (Sequence::Trace, &common::STEPS_TRACE),
        (Sequence::AddressError, &common::STEPS_ADDRESS_ERROR),
        (Sequence::DivideZero, &common::STEPS_DIV_ZERO),
        (Sequence::Privilege, &common::STEPS_PRIVILEGE_VIOLATION),
        (Sequence::BranchTaken, &bcc::STEPS_BRANCH_TAKEN),
        (Sequence::BranchShort, &bcc::STEPS_BRANCH_NOT_TAKEN_SHORT),
        (Sequence::BranchWord, &bcc::STEPS_BRANCH_NOT_TAKEN_WORD),
        (Sequence::DbTrue, &dbcc::STEPS_DBCC_COND_TRUE),
        (Sequence::DbTaken, &dbcc::STEPS_DBCC_BRANCH_TAKEN),
        (Sequence::DbExpired, &dbcc::STEPS_DBCC_COUNTER_EXPIRED),
        (Sequence::SccTrue, &scc::STEPS_SCC_DN_TRUE),
        (Sequence::SccFalse, &scc::STEPS_SCC_DN_FALSE),
        (Sequence::ChkNegative, &chk::STEPS_CHK_TRAP),
        (Sequence::ChkUpper, &chk::STEPS_CHK_TRAP[1..]),
        (Sequence::TrapOverflow, &trapv::STEPS_TRAPV_EXCEPTION),
    ]
}

pub fn serialize<S: Serializer>(
    steps: &&'static [MicroStep],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    for (id, candidate) in redirected() {
        if *steps == candidate {
            return id.serialize(serializer);
        }
    }
    for (opcode, descriptor) in OPCODE_DESCRIPTOR_TABLE.iter().enumerate() {
        if *steps == descriptor.steps {
            return Sequence::Opcode(opcode as u16).serialize(serializer);
        }
    }
    Err(serde::ser::Error::custom("Unknown CPU execution sequence"))
}

pub fn deserialize<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<&'static [MicroStep], D::Error> {
    let id = Sequence::deserialize(deserializer)?;
    if let Sequence::Opcode(opcode) = id {
        return Ok(OPCODE_DESCRIPTOR_TABLE[usize::from(opcode)].steps);
    }
    // Comparing serialized enum discriminants would couple the schema to layout.
    let steps = match id {
        Sequence::Empty => &[][..],
        Sequence::Interrupt => &common::STEPS_INTERRUPT,
        Sequence::Trace => &common::STEPS_TRACE,
        Sequence::AddressError => &common::STEPS_ADDRESS_ERROR,
        Sequence::DivideZero => &common::STEPS_DIV_ZERO,
        Sequence::Privilege => &common::STEPS_PRIVILEGE_VIOLATION,
        Sequence::BranchTaken => &bcc::STEPS_BRANCH_TAKEN,
        Sequence::BranchShort => &bcc::STEPS_BRANCH_NOT_TAKEN_SHORT,
        Sequence::BranchWord => &bcc::STEPS_BRANCH_NOT_TAKEN_WORD,
        Sequence::DbTrue => &dbcc::STEPS_DBCC_COND_TRUE,
        Sequence::DbTaken => &dbcc::STEPS_DBCC_BRANCH_TAKEN,
        Sequence::DbExpired => &dbcc::STEPS_DBCC_COUNTER_EXPIRED,
        Sequence::SccTrue => &scc::STEPS_SCC_DN_TRUE,
        Sequence::SccFalse => &scc::STEPS_SCC_DN_FALSE,
        Sequence::ChkNegative => &chk::STEPS_CHK_TRAP,
        Sequence::ChkUpper => &chk::STEPS_CHK_TRAP[1..],
        Sequence::TrapOverflow => &trapv::STEPS_TRAPV_EXCEPTION,
        Sequence::Opcode(_) => return Err(serde::de::Error::custom("Invalid sequence")),
    };
    Ok(steps)
}
