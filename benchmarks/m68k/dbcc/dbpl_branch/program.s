; DBPL has a false condition and a live counter.
; A missed branch executes the ILLEGAL sentinel.
; MOVE.L reloads the counter; MOVE.W sets CCR for the next target.
entry:
    rept {{instruction_count}}
    dbpl d5,*+6
    dc.w $4AFC
    move.l d6,d5
    move.w #$0008,ccr
    dbpl d5,*+6
    dc.w $4AFC
    move.l d6,d5
    move.w #$0008,ccr
    dbpl d5,*+6
    dc.w $4AFC
    move.l d6,d5
    move.w #$0008,ccr
    dbpl d5,*+6
    dc.w $4AFC
    move.l d6,d5
    move.w #$0008,ccr
    endr
    dc.b "M68K_END_MARKER!"
