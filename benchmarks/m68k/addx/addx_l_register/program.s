; Repeat the register form four times with a MOVE helper after each target.
entry:
    rept {{instruction_count}}
    addx.l d3,d4
    move.l d7,d0
    addx.l d3,d4
    move.l d7,d0
    addx.l d3,d4
    move.l d7,d0
    addx.l d3,d4
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
