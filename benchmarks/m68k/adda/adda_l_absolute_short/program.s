; Restore A6 after each ADDA target; source EA remains in range.
entry:
    rept {{instruction_count}}
    adda.l $1000.w,a6
    movea.l d7,a6
    adda.l $1000.w,a6
    movea.l d7,a6
    adda.l $1000.w,a6
    movea.l d7,a6
    adda.l $1000.w,a6
    movea.l d7,a6
    endr
    dc.b "M68K_END_MARKER!"
