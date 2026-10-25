; DBF is always false. Decrementing zero to $FFFF ends the loop.
; An incorrect branch at the first or last target reaches ILLEGAL.
    dc.w $4AFC
entry:
    rept {{instruction_count}}
    dbf d5,*-2
    move.l d6,d5
    move.w #$0000,ccr
    dbf d5,*+10
    move.l d6,d5
    move.w #$0000,ccr
    dbf d5,*+10
    move.l d6,d5
    move.w #$0000,ccr
    dbf d5,*+26
    move.l d6,d5
    move.w #$0000,ccr
    endr
    dc.b "M68K_END_MARKER!"
    dc.w $4AFC
