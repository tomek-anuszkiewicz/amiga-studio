; MOVE.W #$0000,CCR keeps Z clear, so every measured BEQ.W falls through.
; If taken, the first BEQ of the first repetition reaches the leading ILLEGAL,
; and the fourth BEQ of the last repetition reaches the ILLEGAL after the marker.
    dc.w $4AFC
entry:
    rept {{instruction_count}}
    beq.w *-2
    move.w #$0000,ccr
    beq.w *+8
    move.w #$0000,ccr
    beq.w *+8
    move.w #$0000,ccr
    beq.w *+24
    move.w #$0000,ccr
    endr
    dc.b "M68K_END_MARKER!"
    dc.w $4AFC
