; Restore SR after each target to keep its supervisor and condition-code input fixed.
; An inert MOVE.L precedes each status or USP restoration.
entry:
    rept {{instruction_count}}
    eori.b #$11,ccr
    move.l d0,d1
    move.w #$2704,sr
    eori.b #$11,ccr
    move.l d0,d1
    move.w #$2704,sr
    eori.b #$11,ccr
    move.l d0,d1
    move.w #$2704,sr
    eori.b #$11,ccr
    move.l d0,d1
    move.w #$2704,sr
    endr
    dc.b "M68K_END_MARKER!"
