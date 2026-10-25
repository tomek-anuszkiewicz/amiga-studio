destination_data:
    dcb.w 100000,$a55a
entry:
    rept {{instruction_count}}
    move.w sr,-(a7)
    move.l d0,d1
    move.w sr,-(a7)
    move.l d0,d1
    move.w sr,-(a7)
    move.l d0,d1
    move.w sr,-(a7)
    move.l d0,d1
    endr
    dc.b "M68K_END_MARKER!"
