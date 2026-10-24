; Measure NEGX.W with the selected destination mode.
entry:
    rept {{instruction_count}}
    negx.w $1000.w
    move.l d7,d0
    negx.w $1000.w
    move.l d7,d0
    negx.w $1000.w
    move.l d7,d0
    negx.w $1000.w
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
