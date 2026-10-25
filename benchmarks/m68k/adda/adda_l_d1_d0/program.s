; Restore A6 after each ADDA target; source EA remains in range.
entry:
    rept {{instruction_count}}
    adda.l d0,a6
    movea.l d7,a6
    adda.l d2,a6
    movea.l d7,a6
    adda.l d4,a6
    movea.l d7,a6
    adda.l d0,a6
    movea.l d7,a6
    endr
    dc.b "M68K_END_MARKER!"
