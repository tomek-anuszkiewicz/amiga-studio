; Restore A7 after each ADDA target; source EA remains in range.
entry:
    rept {{instruction_count}}
    adda.w d4,a7
    movea.l d7,a7
    adda.w d5,a7
    movea.l d7,a7
    adda.w d6,a7
    movea.l d7,a7
    adda.w d7,a7
    movea.l d7,a7
    endr
    dc.b "M68K_END_MARKER!"
