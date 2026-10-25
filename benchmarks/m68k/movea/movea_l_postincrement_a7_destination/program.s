; Load A7 from the selected source EA, then restore it.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    movea.l (a1)+,a7
    movea.l d7,a7
    movea.l (a1)+,a7
    movea.l d7,a7
    movea.l (a1)+,a7
    movea.l d7,a7
    movea.l (a1)+,a7
    movea.l d7,a7
    endr
    dc.b "M68K_END_MARKER!"
