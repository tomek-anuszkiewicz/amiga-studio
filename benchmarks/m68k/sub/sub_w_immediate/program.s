; Measure the SUB.W source effective address with four interleaved targets.
entry:
    rept {{instruction_count}}
    sub.w #$1234,d0
    move.l d7,d1
    sub.w #$1234,d0
    move.l d7,d1
    sub.w #$1234,d0
    move.l d7,d1
    sub.w #$1234,d0
    move.l d7,d1
    endr
    dc.b "M68K_END_MARKER!"
