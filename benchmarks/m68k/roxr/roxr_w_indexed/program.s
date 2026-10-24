; Update four fixed words through separate base and index registers.
    dc.w $1357,$246B,$369D
    dc.w $6B7D,$C39E,$2D5F,$B8E7
entry:
    rept {{instruction_count}}
    roxr.w 6(a0,d0.w)
    move.l d6,d7
    roxr.w 6(a3,d2.w)
    move.l d6,d7
    roxr.w 6(a4,d3.w)
    move.l d6,d7
    roxr.w 6(a6,d5.w)
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
