; Repeat indexed read-modify-write on four words at fixed addresses.
    dc.w $1357,$246B,$369D
    dc.w $6B7D,$C39E,$2D5F,$B8E7
entry:
    rept {{instruction_count}}
    ori.w #$5A3C,6(a0,d0.w)
    move.l d6,d7
    ori.w #$5A3C,6(a1,d2.w)
    move.l d6,d7
    ori.w #$5A3C,6(a5,d4.w)
    move.l d6,d7
    ori.w #$5A3C,6(a6,d5.w)
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
