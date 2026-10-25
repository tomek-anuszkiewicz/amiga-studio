; MOVE.L reloads a nonzero counter; MOVE.W keeps Z clear, so NE is true.
; Every DBNE falls through without decrementing D5. If it branched, the first
; target of the first repetition and the fourth of the last reach ILLEGAL.
    dc.w $4AFC
entry:
    rept {{instruction_count}}
    dbne d5,*-2
    move.l d6,d5
    move.w #$0000,ccr
    dbne d5,*+10
    move.l d6,d5
    move.w #$0000,ccr
    dbne d5,*+10
    move.l d6,d5
    move.w #$0000,ccr
    dbne d5,*+26
    move.l d6,d5
    move.w #$0000,ccr
    endr
    dc.b "M68K_END_MARKER!"
    dc.w $4AFC
