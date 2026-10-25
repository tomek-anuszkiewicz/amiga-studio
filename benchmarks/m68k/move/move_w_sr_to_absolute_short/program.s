entry:
    rept {{instruction_count}}
    move.w sr,$2000.w
    move.l d0,d1
    move.w sr,$2000.w
    move.l d0,d1
    move.w sr,$2000.w
    move.l d0,d1
    move.w sr,$2000.w
    move.l d0,d1
    endr
    dc.b "M68K_END_MARKER!"
