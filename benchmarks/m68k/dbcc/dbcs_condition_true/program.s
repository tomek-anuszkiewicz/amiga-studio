; DBCS: The condition is true, so the counter is untouched.
; An incorrect branch at the first or last target reaches ILLEGAL.
; MOVE.L reloads the counter; MOVE.W sets CCR for the next target.
    dc.w $4AFC
entry:
    rept {{instruction_count}}
    dbcs d5,*-2
    move.l d6,d5
    move.w #$0001,ccr
    dbcs d5,*+10
    move.l d6,d5
    move.w #$0001,ccr
    dbcs d5,*+10
    move.l d6,d5
    move.w #$0001,ccr
    dbcs d5,*+26
    move.l d6,d5
    move.w #$0001,ccr
    endr
    dc.b "M68K_END_MARKER!"
    dc.w $4AFC
