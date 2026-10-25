; MOVE.W #$0000,CCR sets the intended flags. DBCC condition is true.
; An incorrect branch reaches an ILLEGAL sentinel at the first
; target or after the end marker at the final target.
    dc.w $4AFC
entry:
    rept {{instruction_count}}
    dbcc d5,*-2
    move.l d6,d5
    move.w #$0000,ccr
    dbcc d5,*+10
    move.l d6,d5
    move.w #$0000,ccr
    dbcc d5,*+10
    move.l d6,d5
    move.w #$0000,ccr
    dbcc d5,*+26
    move.l d6,d5
    move.w #$0000,ccr
    endr
    dc.b "M68K_END_MARKER!"
    dc.w $4AFC
