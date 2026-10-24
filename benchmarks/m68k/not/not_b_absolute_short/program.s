; Measure NOT.B with the selected destination mode.
entry:
    rept {{instruction_count}}
    not.b $1000.w
    move.l d7,d0
    not.b $1000.w
    move.l d7,d0
    not.b $1000.w
    move.l d7,d0
    not.b $1000.w
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
