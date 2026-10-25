; Four LEA (14,A5,D2.W),A4 targets, each followed by a MOVE helper.
entry:
    rept {{instruction_count}}
    lea (14,a5,d2.w),a4
    move.l d6,d7
    lea (14,a5,d2.w),a4
    move.l d6,d7
    lea (14,a5,d2.w),a4
    move.l d6,d7
    lea (14,a5,d2.w),a4
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
