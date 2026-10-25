; SNE writes the fixed word at the absolute short address.
entry:
    rept {{instruction_count}}
    sne $1000.w
    move.l d6,d7
    sne $1000.w
    move.l d6,d7
    sne $1000.w
    move.l d6,d7
    sne $1000.w
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
