; DBGT: The false condition decrements zero to $FFFF and falls through.
; An incorrect branch at the first or last target reaches ILLEGAL.
; MOVE.L reloads the counter; MOVE.W sets CCR for the next target.
    dc.w $4AFC
entry:
    rept {{instruction_count}}
    dbgt d5,*-2
    move.l d6,d5
    move.w #$0004,ccr
    dbgt d5,*+10
    move.l d6,d5
    move.w #$0004,ccr
    dbgt d5,*+10
    move.l d6,d5
    move.w #$0004,ccr
    dbgt d5,*+26
    move.l d6,d5
    move.w #$0004,ccr
    endr
    dc.b "M68K_END_MARKER!"
    dc.w $4AFC
