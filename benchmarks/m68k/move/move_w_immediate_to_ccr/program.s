; Move the selected source word to CCR in supervisor mode.
entry:
    rept {{instruction_count}}
    move.w #$5a3c,ccr
    move.w d7,sr
    move.w #$5a3c,ccr
    move.w d7,sr
    move.w #$5a3c,ccr
    move.w d7,sr
    move.w #$5a3c,ccr
    move.w d7,sr
    endr
    dc.b "M68K_END_MARKER!"
