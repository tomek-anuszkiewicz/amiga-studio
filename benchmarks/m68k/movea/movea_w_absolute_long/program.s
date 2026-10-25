; Load A6 from the selected source EA, then restore it.
operand_data:
    dc.w $1234
entry:
    rept {{instruction_count}}
    movea.w $200000.l,a6
    movea.l d7,a6
    movea.w $200000.l,a6
    movea.l d7,a6
    movea.w $200000.l,a6
    movea.l d7,a6
    movea.w $200000.l,a6
    movea.l d7,a6
    endr
    dc.b "M68K_END_MARKER!"
