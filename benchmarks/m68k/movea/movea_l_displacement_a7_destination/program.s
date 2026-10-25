; Load A7 from the selected source EA, then restore it.
    dc.w $1357
    dc.w $246B
    dc.w $369D
operand_data:
    dc.l $12345679
entry:
    rept {{instruction_count}}
    movea.l 6(a3),a7
    movea.l d7,a7
    movea.l 6(a3),a7
    movea.l d7,a7
    movea.l 6(a3),a7
    movea.l d7,a7
    movea.l 6(a3),a7
    movea.l d7,a7
    endr
    dc.b "M68K_END_MARKER!"
