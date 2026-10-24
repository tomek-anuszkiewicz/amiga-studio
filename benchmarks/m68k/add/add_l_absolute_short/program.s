; Measure the ADD.L source effective address with four interleaved targets.
entry:
    rept {{instruction_count}}
    add.l $1000.w,d2
    move.l d7,d0
    add.l $1000.w,d2
    move.l d7,d0
    add.l $1000.w,d2
    move.l d7,d0
    add.l $1000.w,d2
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
