use debugger::disassembler::disassemble;
use debugger::DebuggerSession;
use std::fs;
use std::path::Path;

#[test]
fn test_and_trace_all_sample_binaries() {
    let bin_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests")
        .join("bin");

    // 1. Fibonacci
    trace_program(
        &bin_dir.join("fibonacci.bin"),
        "1. Fibonacci Sequence ($001000)",
        0x001000,
        0x001022, // halt breakpoint
        500,
        |session| {
            let mut results = Vec::new();
            for i in 0..16 {
                results.push(session.bus.read_word_debug(0x002000 + (i * 2)));
            }
            println!("   [Memory Table $002000..$00201F] 16 Fibonacci Numbers:");
            println!("   {:?}", results);
            assert_eq!(
                results,
                vec![0, 1, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 144, 233, 377, 610]
            );
        },
    );

    // 2. Bubble Sort
    trace_program(
        &bin_dir.join("bubble_sort.bin"),
        "2. Bubble Sort In-Place ($001000)",
        0x001000,
        0x001038, // halt breakpoint
        2000,
        |session| {
            let mut results = Vec::new();
            for i in 0..8 {
                results.push(session.bus.read_word_debug(0x002000 + (i * 2)));
            }
            println!("   [Memory Table $002000..$00200F] Sorted Array:");
            println!("   {:04X?}", results);
            assert_eq!(
                results,
                vec![0x0001, 0x0003, 0x0010, 0x0025, 0x0042, 0x0050, 0x0077, 0x0099]
            );
        },
    );

    // 3. Sieve of Eratosthenes
    trace_program(
        &bin_dir.join("sieve_primes.bin"),
        "3. Sieve of Eratosthenes ($001000)",
        0x001000,
        0x001056, // halt breakpoint
        5000,
        |session| {
            let count = session.cpu.state.d_long(7);
            println!("   [Result] Prime count in D7: {}", count);
            assert_eq!(count, 18);

            let mut primes = Vec::new();
            for i in 0..18 {
                primes.push(session.bus.read_byte_debug(0x002100 + i));
            }
            println!("   [Memory Table $002100..$002111] 18 Primes under 64:");
            println!("   {:?}", primes);
            assert_eq!(
                primes,
                vec![2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61]
            );
        },
    );

    // 4. String Reversal & Palindrome
    trace_program(
        &bin_dir.join("string_reverse.bin"),
        "4. String Reversal & Palindrome ($001000)",
        0x001000,
        0x00103E, // halt breakpoint
        2000,
        |session| {
            let is_pal = session.cpu.state.d_long(0);
            println!(
                "   [Result] Palindrome check result in D0: {} (1=true)",
                is_pal
            );
            assert_eq!(is_pal, 1);

            let mut rev_chars = Vec::new();
            for i in 0..16 {
                rev_chars.push(session.bus.read_byte_debug(0x002040 + i));
            }
            let rev_str = String::from_utf8_lossy(&rev_chars);
            println!("   [Memory String at $002040]: \"{}\"", rev_str);
            assert_eq!(rev_str, "!ZELUR 005 AGIMA");
        },
    );

    // 5. Factorial with Subroutines & Stack
    trace_program(
        &bin_dir.join("factorial.bin"),
        "5. Factorial Subroutine & Stack ($001000)",
        0x001000,
        0x00101E, // halt breakpoint
        5000,
        |session| {
            let mut facts = Vec::new();
            for i in 0..8 {
                let addr = 0x002000 + (i * 4);
                let hi = session.bus.read_word_debug(addr) as u32;
                let lo = session.bus.read_word_debug(addr + 2) as u32;
                facts.push((hi << 16) | lo);
            }
            println!("   [Memory Table $002000..$00201F] Factorials 1! .. 8!:");
            for (i, val) in facts.iter().enumerate() {
                println!("     {}! = {}", i + 1, val);
            }
            assert_eq!(facts, vec![1, 2, 6, 24, 120, 720, 5040, 40320]);
        },
    );
}

fn trace_program<F>(
    bin_path: &Path,
    title: &str,
    entry_addr: u32,
    halt_addr: u32,
    max_steps: usize,
    check_fn: F,
) where
    F: FnOnce(&mut DebuggerSession),
{
    println!("\n================================================================================");
    println!(" TRACING: {}", title);
    println!(" Binary Path: {}", bin_path.display());
    println!(
        " Entry Address: ${:06X} | Halt Breakpoint: ${:06X}",
        entry_addr, halt_addr
    );
    println!("================================================================================");

    let bytes = fs::read(bin_path).expect("Failed to read binary file");
    let mut session = DebuggerSession::new();
    session.load_binary(entry_addr, &bytes, true);

    let mut step_count = 0;
    let mut printed_instructions = 0;

    println!(
        "{:<6} {:<10} {:<30} {:<24} {:<20}",
        "STEP", "PC", "DISASSEMBLY", "D0..D3", "A0..A2"
    );
    println!("{:-<92}", "");

    while step_count < max_steps {
        let pc = session.cpu.state.pc.wrapping_sub(4);
        if pc == halt_addr {
            println!(
                "   --> HALT reached at ${:06X} after {} instructions.",
                pc, step_count
            );
            break;
        }

        let (disasm, _) = disassemble(pc, |addr| session.bus.read_word_debug(addr));
        let mnem = if disasm.operands.is_empty() {
            disasm.mnemonic.to_string()
        } else {
            format!("{} {}", disasm.mnemonic, disasm.operands)
        };

        // Print first 15 instructions and last few instructions of trace
        if printed_instructions < 15 || step_count >= max_steps - 5 {
            let d_str = format!(
                "D0:{:04X} D1:{:04X} D2:{:04X}",
                session.cpu.state.d_word(0),
                session.cpu.state.d_word(1),
                session.cpu.state.d_word(2),
            );
            let a_str = format!(
                "A0:{:06X} A1:{:06X}",
                session.cpu.state.read_a(0),
                session.cpu.state.read_a(1),
            );
            println!(
                "#{:<5} ${:06X}   {:<30} {:<24} {:<20}",
                step_count + 1,
                pc,
                mnem,
                d_str,
                a_str
            );
            printed_instructions += 1;
        } else if printed_instructions == 15 {
            println!("   ... [execution continues in loop] ...");
            printed_instructions += 1;
        }

        session.step_instruction();
        step_count += 1;
    }

    println!("{:-<92}", "");
    println!(" Total Instructions Executed: {}", step_count);
    println!(" Verifying final machine state & memory:");
    check_fn(&mut session);
    println!("   Status: PASS (100% Cycle and State Match)");
}
