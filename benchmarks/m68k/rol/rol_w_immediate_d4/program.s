; Rotate a register word left by a fixed nonzero count.
entry:
    rept {{instruction_count}}
    rol.w #3,d4
    move.l d7,d0
    rol.w #3,d4
    move.l d7,d0
    rol.w #3,d4
    move.l d7,d0
    rol.w #3,d4
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
