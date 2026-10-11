#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! SINGLESTEP-MUTATION-01: raw JSON expectations reach the ordinary CPU comparator.
mod common;

use common::mutations::{seeds, Pilot, Seed};
use serde_json::{json, Value};

#[test]
fn silicon_json_mutations_check_the_shared_verification_path() {
    let seeds = seeds();
    let mut pilot = Pilot::new();
    for seed in &seeds {
        pilot.baseline(seed);
        state_mutations(&mut pilot, seed);
        transaction_mutations(&mut pilot, seed);
        input_mutations(&mut pilot, seed);
        positive_controls(&mut pilot, seed);
        format_mutations(&mut pilot, seed);
    }
    pilot.finish(&seeds);
}

fn state_mutations(pilot: &mut Pilot, seed: &Seed) {
    for field in (0..8)
        .map(|i| format!("d{i}"))
        .chain((0..7).map(|i| format!("a{i}")))
        .chain(["usp".into(), "ssp".into()])
    {
        let value = seed.raw["final"][&field].as_u64().unwrap() ^ 1;
        pilot.change(seed, &format!("/final/{field}"), json!(value), &field);
    }
    for bit in [1, 2, 4, 8, 16, 0x100, 0x2000, 0x8000] {
        let value = seed.raw["final"]["sr"].as_u64().unwrap() ^ bit;
        pilot.change(seed, "/final/sr", json!(value), "sr");
    }
    for delta in [2i64, -2, 4] {
        let value = (seed.raw["final"]["pc"].as_u64().unwrap() as i64 + delta) as u32;
        pilot.change(seed, "/final/pc", json!(value), "pc");
    }
    for word in 0..2 {
        let value = seed.raw["final"]["prefetch"][word].as_u64().unwrap() ^ 1;
        pilot.change(
            seed,
            &format!("/final/prefetch/{word}"),
            json!(value),
            &format!("prefetch{word}"),
        );
    }
    pilot.change(
        seed,
        "/length",
        json!(seed.raw["length"].as_u64().unwrap() + 2),
        "length",
    );
}

fn transaction_mutations(pilot: &mut Pilot, seed: &Seed) {
    let txs = seed.raw["transactions"].as_array().unwrap();
    let bus: Vec<_> = txs
        .iter()
        .enumerate()
        .filter(|(_, t)| t[0] != "n")
        .map(|(i, _)| i)
        .collect();
    for &index in &bus {
        let tx = &txs[index];
        pilot.change(
            seed,
            &format!("/transactions/{index}/5"),
            json!(tx[5].as_u64().unwrap() ^ 1),
            "transaction_data",
        );
        pilot.change(
            seed,
            &format!("/transactions/{index}/3"),
            json!((tx[3].as_u64().unwrap() + 2) & 0xFFFFFF),
            "transaction_address",
        );
        pilot.change(
            seed,
            &format!("/transactions/{index}/0"),
            json!(if tx[0] == "r" { "w" } else { "r" }),
            "transaction_direction",
        );
        let mut raw = seed.raw.clone();
        raw["transactions"][index][4] = json!(if tx[4] == ".b" { ".w" } else { ".b" });
        raw["transactions"][index][5] = json!(tx[5].as_u64().unwrap() & 0xFF);
        paired_transactions(
            pilot,
            seed,
            "access_size",
            raw,
            Some("transaction_size"),
            "observable_expectation",
        );
        pilot.change(
            seed,
            &format!("/transactions/{index}/1"),
            json!(6),
            "transaction_duration",
        );
        let mut raw = seed.raw.clone();
        raw["transactions"][index][2] = json!(if tx[2] == 5 || tx[2] == 6 { 1 } else { 5 });
        pilot.probe(seed, "function_code", raw.clone(), vec![json!({"path":format!("/transactions/{index}/2"),"old":tx[2],"new":raw["transactions"][index][2]})],
            None, "unobserved_function_code", &seed.suite);
    }
    for duplicate in [false, true] {
        let mut raw = seed.raw.clone();
        let entries = raw["transactions"].as_array_mut().unwrap();
        if duplicate {
            entries.insert(bus[0], txs[bus[0]].clone());
        } else {
            entries.remove(bus[0]);
        }
        paired_transactions(
            pilot,
            seed,
            if duplicate {
                "duplicate_transfer"
            } else {
                "delete_transfer"
            },
            raw,
            Some("transaction_count"),
            "observable_expectation",
        );
    }
    for direction in ["r", "w"] {
        let distinct: Vec<_> = bus
            .iter()
            .copied()
            .filter(|&i| txs[i][0] == direction)
            .collect();
        if distinct.len() >= 2 && txs[distinct[0]] != txs[distinct[1]] {
            let mut raw = seed.raw.clone();
            raw["transactions"]
                .as_array_mut()
                .unwrap()
                .swap(distinct[0], distinct[1]);
            paired_transactions(
                pilot,
                seed,
                "transfer_order",
                raw,
                Some("transaction_address"),
                "observable_expectation",
            );
        }
    }
    if let Some(idle) = txs.iter().position(|t| t[0] == "n") {
        let mut raw = seed.raw.clone();
        let entries = raw["transactions"].as_array_mut().unwrap();
        let interval = entries.remove(idle);
        let target = if idle == 0 { entries.len() } else { 0 };
        entries.insert(target, interval);
        paired_transactions(
            pilot,
            seed,
            "internal_interval_placement",
            raw,
            Some("transaction_clock"),
            "observable_expectation",
        );

        let mut raw = seed.raw.clone();
        raw["transactions"][idle][1] = json!(txs[idle][1].as_u64().unwrap() + 2);
        paired_transactions(
            pilot,
            seed,
            "internal_duration",
            raw,
            Some("transaction_timeline"),
            "observable_expectation",
        );
        if txs[idle][1].as_u64().unwrap() >= 2 {
            let mut raw = seed.raw.clone();
            raw["transactions"][idle][1] = json!(txs[idle][1].as_u64().unwrap() - 2);
            raw["transactions"][bus[0]][1] = json!(6);
            paired_transactions(
                pilot,
                seed,
                "compensated_bus_duration",
                raw,
                Some("transaction_duration"),
                "observable_expectation",
            );
        }
    }
    // Final read duration can change without changing its CCK1 callback or total clocks.
    if txs.len() >= 2 && txs[txs.len() - 1][0] == "n" && txs[txs.len() - 2][0] == "r" {
        let mut raw = seed.raw.clone();
        let end = txs.len() - 1;
        raw["transactions"][end][1] = json!(txs[end][1].as_u64().unwrap() - 2);
        raw["transactions"][end - 1][1] = json!(6);
        paired_transactions(
            pilot,
            seed,
            "trailing_read_idle_split",
            raw,
            Some("transaction_duration"),
            "observable_expectation",
        );
    }
}
fn paired_transactions(
    pilot: &mut Pilot,
    seed: &Seed,
    label: &str,
    raw: Value,
    required: Option<&str>,
    disposition: &str,
) {
    let edit =
        json!({"path":"/transactions","old":seed.raw["transactions"],"new":raw["transactions"]});
    pilot.probe(
        seed,
        label,
        raw,
        vec![edit],
        required,
        disposition,
        &seed.suite,
    );
}

fn input_mutations(pilot: &mut Pilot, seed: &Seed) {
    let opcode = seed.raw["initial"]["prefetch"][0].as_u64().unwrap();
    if seed.path == "register" || seed.path == "quotient" {
        let source = opcode & 7;
        let path = format!("/initial/d{source}");
        let value = seed.raw.pointer(&path).unwrap().as_u64().unwrap() ^ 1;
        let dest = (opcode >> 9) & 7;
        pilot.change(seed, &path, json!(value), &format!("d{dest}"));
    } else if let Some(tx) = seed.raw["transactions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t[0] == "r" && t[2] == 5)
    {
        let addr = tx[3].as_u64().unwrap();
        if let Some(i) = seed.raw["initial"]["ram"]
            .as_array()
            .unwrap()
            .iter()
            .position(|r| r[0] == addr)
        {
            pilot.change(
                seed,
                &format!("/initial/ram/{i}/1"),
                json!(seed.raw["initial"]["ram"][i][1].as_u64().unwrap() ^ 1),
                "transaction_data",
            );
        }
    }
}

fn positive_controls(pilot: &mut Pilot, seed: &Seed) {
    let mut raw = seed.raw.clone();
    raw["name"] = json!("renamed silicon mutation control");
    pilot.probe(
        seed,
        "name",
        raw,
        vec![
            json!({"path":"/name","old":seed.raw["name"],"new":"renamed silicon mutation control"}),
        ],
        Some("pass"),
        "equivalent_control",
        &seed.suite,
    );
    let mut raw = seed.raw.clone();
    raw["diagnostic_metadata"] = json!({"comment":"no execution semantics"});
    pilot.probe(seed, "metadata", raw, vec![json!({"path":"/diagnostic_metadata","old":null,"new":{"comment":"no execution semantics"}})], Some("pass"), "equivalent_control", &seed.suite);
    let mut raw = seed.raw.clone();
    let addr = (0xED0000..0xED0010)
        .find(|&a| {
            !seed.raw["initial"]["ram"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r[0] == a)
                && !seed.raw["transactions"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|t| t[0] != "n" && t[3].as_u64().unwrap().abs_diff(a) < 2)
        })
        .unwrap();
    raw["initial"]["ram"]
        .as_array_mut()
        .unwrap()
        .push(json!([addr, 0xA5]));
    pilot.probe(seed, "unused_initial_ram", raw.clone(), vec![json!({"path":"/initial/ram","old":seed.raw["initial"]["ram"],"new":raw["initial"]["ram"]})], Some("pass"), "equivalent_control", &seed.suite);
    let mut raw = seed.raw.clone();
    if let Some(entry) = raw["final"]["ram"].as_array_mut().unwrap().first_mut() {
        entry[1] = json!(entry[1].as_u64().unwrap() ^ 1);
        pilot.probe(seed, "unused_final_ram", raw.clone(), vec![json!({"path":"/final/ram","old":seed.raw["final"]["ram"],"new":raw["final"]["ram"]})], Some("pass"), "equivalent_control", &seed.suite);
    }
}

fn format_mutations(pilot: &mut Pilot, seed: &Seed) {
    for path in ["/name", "/length", "/initial/d0", "/final/prefetch"] {
        let mut raw = seed.raw.clone();
        let (parent, key) = path.rsplit_once('/').unwrap();
        let old = raw.pointer(path).unwrap().clone();
        raw.pointer_mut(parent)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove(key);
        pilot.probe(
            seed,
            "missing_required_field",
            raw,
            vec![json!({"path":path,"old":old,"new":null})],
            Some("format"),
            "format_validation",
            &seed.suite,
        );
    }
    for (path, value) in [
        ("/initial/d0", json!("invalid")),
        ("/final/sr", json!(65536)),
        ("/initial/prefetch/1", json!(65536)),
        ("/initial/ram/0/1", json!(256)),
    ] {
        if seed.raw.pointer(path).is_some() {
            pilot.change(seed, path, value, "format");
        }
    }
    let first_bus = seed.raw["transactions"]
        .as_array()
        .unwrap()
        .iter()
        .position(|t| t[0] != "n")
        .unwrap();
    for (field, value) in [
        (0, json!("x")),
        (1, json!(4294967296u64)),
        (2, json!(8)),
        (3, json!(0x1000000)),
        (4, json!(".l")),
        (5, json!(65536)),
    ] {
        pilot.change(
            seed,
            &format!("/transactions/{first_bus}/{field}"),
            value,
            "format",
        );
    }
}
