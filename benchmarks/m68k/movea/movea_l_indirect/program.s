; Load A6 from the selected source EA, then restore it.
operand_data:
    dc.l $12345679
entry:
    rept {{instruction_count}}
    movea.l (a3),a6
    movea.l d7,a6
    movea.l (a3),a6
    movea.l d7,a6
    movea.l (a3),a6
    movea.l d7,a6
    movea.l (a3),a6
    movea.l d7,a6
    endr
    dc.b "M68K_END_MARKER!"
