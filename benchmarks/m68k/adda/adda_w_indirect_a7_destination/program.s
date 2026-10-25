; Restore A7 after each ADDA target; source EA remains in range.
operand_data:
    dc.w $1234
entry:
    rept {{instruction_count}}
    adda.w (a3),a7
    movea.l d7,a7
    adda.w (a3),a7
    movea.l d7,a7
    adda.w (a3),a7
    movea.l d7,a7
    adda.w (a3),a7
    movea.l d7,a7
    endr
    dc.b "M68K_END_MARKER!"
