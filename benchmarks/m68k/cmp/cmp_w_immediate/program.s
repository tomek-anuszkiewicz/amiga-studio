; Measure CMP.W #$1234,D0 without changing the destination.
entry:
    rept {{instruction_count}}
    cmp.w #$1234,d0
    move.l d7,d1
    cmp.w #$1234,d0
    move.l d7,d1
    cmp.w #$1234,d0
    move.l d7,d1
    cmp.w #$1234,d0
    move.l d7,d1
    endr
    dc.b "M68K_END_MARKER!"
