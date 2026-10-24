; Measure the SUB.L source effective address with four interleaved targets.
entry:
    rept {{instruction_count}}
    sub.l #$12345679,d0
    move.l d7,d1
    sub.l #$12345679,d0
    move.l d7,d1
    sub.l #$12345679,d0
    move.l d7,d1
    sub.l #$12345679,d0
    move.l d7,d1
    endr
    dc.b "M68K_END_MARKER!"
