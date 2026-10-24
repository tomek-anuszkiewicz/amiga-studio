; Repeat the register form four times with a MOVE helper after each target.
entry:
    rept {{instruction_count}}
    chk.w d3,d4
    move.l d7,d0
    chk.w d3,d4
    move.l d7,d0
    chk.w d3,d4
    move.l d7,d0
    chk.w d3,d4
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
