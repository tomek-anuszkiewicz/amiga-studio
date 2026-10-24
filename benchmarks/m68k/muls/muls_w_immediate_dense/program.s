; Four dense immediate multiplies with a restored destination each time.
entry:
    rept {{instruction_count}}
    muls.w #$6b5d,d5
    move.l d6,d5
    muls.w #$6b5d,d5
    move.l d6,d5
    muls.w #$6b5d,d5
    move.l d6,d5
    muls.w #$6b5d,d5
    move.l d6,d5
    endr
    dc.b "M68K_END_MARKER!"
