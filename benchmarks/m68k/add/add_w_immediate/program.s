; Measure the ADD.W source effective address with four interleaved targets.
entry:
    rept {{instruction_count}}
    add.w #$1234,d0
    move.l d7,d1
    add.w #$1234,d0
    move.l d7,d1
    add.w #$1234,d0
    move.l d7,d1
    add.w #$1234,d0
    move.l d7,d1
    endr
    dc.b "M68K_END_MARKER!"
