; Restore A6 after each ADDA target; source EA remains in range.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    adda.l (a1)+,a6
    movea.l d7,a6
    adda.l (a1)+,a6
    movea.l d7,a6
    adda.l (a1)+,a6
    movea.l d7,a6
    adda.l (a1)+,a6
    movea.l d7,a6
    endr
    dc.b "M68K_END_MARKER!"
