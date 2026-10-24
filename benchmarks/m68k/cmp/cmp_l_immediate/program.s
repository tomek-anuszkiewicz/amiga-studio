; Measure CMP.L #$12345679,D0 without changing the destination.
entry:
    rept {{instruction_count}}
    cmp.l #$12345679,d0
    move.l d7,d1
    cmp.l #$12345679,d0
    move.l d7,d1
    cmp.l #$12345679,d0
    move.l d7,d1
    cmp.l #$12345679,d0
    move.l d7,d1
    endr
    dc.b "M68K_END_MARKER!"
