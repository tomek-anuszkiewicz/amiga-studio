; Restore SR after each read so the transferred status word is stable.
; An inert MOVE.L precedes each status or USP restoration.
destination_data:
    dcb.w 100000,$a55a
entry:
    rept {{instruction_count}}
    move.w sr,-(a7)
    move.l d0,d1
    move.w d7,sr
    move.w sr,-(a7)
    move.l d0,d1
    move.w d7,sr
    move.w sr,-(a7)
    move.l d0,d1
    move.w d7,sr
    move.w sr,-(a7)
    move.l d0,d1
    move.w d7,sr
    endr
    dc.b "M68K_END_MARKER!"
