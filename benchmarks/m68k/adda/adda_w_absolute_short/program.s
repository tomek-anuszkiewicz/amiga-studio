; Restore A6 after each ADDA target; source EA remains in range.
; An inert MOVE.L precedes each address-register restoration.
entry:
    rept {{instruction_count}}
    adda.w $1000.w,a6
    move.l d7,d0
    movea.l d7,a6
    adda.w $1000.w,a6
    move.l d7,d0
    movea.l d7,a6
    adda.w $1000.w,a6
    move.l d7,d0
    movea.l d7,a6
    adda.w $1000.w,a6
    move.l d7,d0
    movea.l d7,a6
    endr
    dc.b "M68K_END_MARKER!"
