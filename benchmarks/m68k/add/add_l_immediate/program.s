; Measure the ADD.L source effective address with four interleaved targets.
entry:
    rept {{instruction_count}}
    add.l #$12345679,d0
    move.l d7,d1
    add.l #$12345679,d0
    move.l d7,d1
    add.l #$12345679,d0
    move.l d7,d1
    add.l #$12345679,d0
    move.l d7,d1
    endr
    dc.b "M68K_END_MARKER!"
