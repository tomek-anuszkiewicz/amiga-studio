; Move the selected source word to CCR in supervisor mode.
; An inert MOVE.L precedes each status or USP restoration.
entry:
    rept {{instruction_count}}
    move.w $1000.w,ccr
    move.l d0,d1
    move.w d7,sr
    move.w $1000.w,ccr
    move.l d0,d1
    move.w d7,sr
    move.w $1000.w,ccr
    move.l d0,d1
    move.w d7,sr
    move.w $1000.w,ccr
    move.l d0,d1
    move.w d7,sr
    endr
    dc.b "M68K_END_MARKER!"
