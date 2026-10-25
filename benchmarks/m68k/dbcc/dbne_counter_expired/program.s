; MOVE.L reloads zero; MOVE.W sets Z, so NE is false.
; DBNE decrements zero to $FFFF and falls through when the counter expires.
; If it branched, the first target of the first repetition and the fourth of
; the last reach ILLEGAL.
    dc.w $4AFC
entry:
    rept {{instruction_count}}
    dbne d5,*-2
    move.l d6,d5
    move.w #$0004,ccr
    dbne d5,*+10
    move.l d6,d5
    move.w #$0004,ccr
    dbne d5,*+10
    move.l d6,d5
    move.w #$0004,ccr
    dbne d5,*+26
    move.l d6,d5
    move.w #$0004,ccr
    endr
    dc.b "M68K_END_MARKER!"
    dc.w $4AFC
