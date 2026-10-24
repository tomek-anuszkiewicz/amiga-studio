; Restore SR after each read so the transferred status word is stable.
; An inert MOVE.L precedes each status or USP restoration.
entry:
    rept {{instruction_count}}
    move.w sr,6(a2)
    move.l d0,d1
    move.w d7,sr
    move.w sr,6(a2)
    move.l d0,d1
    move.w d7,sr
    move.w sr,6(a2)
    move.l d0,d1
    move.w d7,sr
    move.w sr,6(a2)
    move.l d0,d1
    move.w d7,sr
    endr
    dc.b "M68K_END_MARKER!"
