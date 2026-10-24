; Restore SR after each target to keep its supervisor and condition-code input fixed.
; An inert MOVE.L precedes each status or USP restoration.
entry:
    rept {{instruction_count}}
    andi.w #$27ef,sr
    move.l d0,d1
    move.w #$271f,sr
    andi.w #$27ef,sr
    move.l d0,d1
    move.w #$271f,sr
    andi.w #$27ef,sr
    move.l d0,d1
    move.w #$271f,sr
    andi.w #$27ef,sr
    move.l d0,d1
    move.w #$271f,sr
    endr
    dc.b "M68K_END_MARKER!"
