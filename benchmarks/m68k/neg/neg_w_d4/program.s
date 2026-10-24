; Negate the same register word on each repetition.
entry:
    rept {{instruction_count}}
    neg.w d4
    move.l d7,d0
    neg.w d4
    move.l d7,d0
    neg.w d4
    move.l d7,d0
    neg.w d4
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
