; Restore the destination so every logical shift starts with set high and low bits.
entry:
    rept {{instruction_count}}
    lsl.w #3,d5
    move.l d7,d5
    lsl.w #3,d5
    move.l d7,d5
    lsl.w #3,d5
    move.l d7,d5
    lsl.w #3,d5
    move.l d7,d5
    endr
    dc.b "M68K_END_MARKER!"
