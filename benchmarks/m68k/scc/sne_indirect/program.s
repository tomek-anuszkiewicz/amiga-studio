; SNE writes $FF through address register indirect; MOVE.L keeps Z clear.
operand_data:
    dc.w $3457
entry:
    rept {{instruction_count}}
    sne (a1)
    move.l d6,d7
    sne (a1)
    move.l d6,d7
    sne (a1)
    move.l d6,d7
    sne (a1)
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
