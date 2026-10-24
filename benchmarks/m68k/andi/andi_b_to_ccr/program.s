; Restore SR after each target to keep its supervisor and condition-code input fixed.
; An inert MOVE.L precedes each status or USP restoration.
entry:
    rept {{instruction_count}}
    andi.b #$0f,ccr
    move.l d0,d1
    move.w #$271f,sr
    andi.b #$0f,ccr
    move.l d0,d1
    move.w #$271f,sr
    andi.b #$0f,ccr
    move.l d0,d1
    move.w #$271f,sr
    andi.b #$0f,ccr
    move.l d0,d1
    move.w #$271f,sr
    endr
    dc.b "M68K_END_MARKER!"
