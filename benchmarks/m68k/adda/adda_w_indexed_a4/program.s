; Read four fixed words through separate base and index registers.
    dc.w $1357,$246B,$369D
    dc.w $1234,$2345,$3456,$4567
entry:
    rept {{instruction_count}}
    adda.w 6(a0,d0.w),a4
    move.l d6,d7
    adda.w 6(a1,d1.w),a4
    move.l d6,d7
    adda.w 6(a2,d2.w),a4
    move.l d6,d7
    adda.w 6(a3,d3.w),a4
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
