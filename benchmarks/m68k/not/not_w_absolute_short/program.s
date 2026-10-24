; Measure NOT.W with the selected destination mode.
entry:
    rept {{instruction_count}}
    not.w $1000.w
    move.l d7,d0
    not.w $1000.w
    move.l d7,d0
    not.w $1000.w
    move.l d7,d0
    not.w $1000.w
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
