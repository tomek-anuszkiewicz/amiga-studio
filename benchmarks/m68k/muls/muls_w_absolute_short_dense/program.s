; Multiply a fixed dense word from absolute short Chip RAM.
entry:
    rept {{instruction_count}}
    muls.w $1000.w,d5
    move.l d6,d5
    muls.w $1000.w,d5
    move.l d6,d5
    muls.w $1000.w,d5
    move.l d6,d5
    muls.w $1000.w,d5
    move.l d6,d5
    endr
    dc.b "M68K_END_MARKER!"
