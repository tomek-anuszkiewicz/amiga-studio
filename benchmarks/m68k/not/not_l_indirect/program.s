; Measure NOT.L with the selected destination mode.
operand_data:
    dc.l $3457689B
entry:
    rept {{instruction_count}}
    not.l (a3)
    move.l d7,d0
    not.l (a3)
    move.l d7,d0
    not.l (a3)
    move.l d7,d0
    not.l (a3)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
