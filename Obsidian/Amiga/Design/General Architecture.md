- Written in Rust
- Emulator must run in WebAssembly (`wasm32-unknown-unknown`) as well as native desktop targets
- Portable across different CPU architectures (x86_64, ARM64, etc.)
- System-agnostic core (no direct OS dependencies in the core emulator engine)
- External injection of save states, memory configurations, and Kickstart ROMs (as raw arrays/slices of bytes `&[u8]`)
- Raw memory is represented throughout the system as arrays of bytes (`[u8]`)
- Flexible execution control: reset emulator, execute single/predefined CCK cycles, or execute full video frame
- Decoupled outputs: methods to retrieve video frame buffer and audio sample buffer


TODO:
- we should avoid connections between modules
- best would be to steer and connect everything in main loop
- we tr simulating real electric cicruit, nothing happen immediatly, so on many place when you change a register, this value would be available to specific chip in next cycle(s), more over if another register is connected to that one, it would some cycles to propagate signal too.
- exaplain how cpu is reading writig memory, how cpu can work with full speed, regarding memory is also used by chips, what is a role of buffers.
  