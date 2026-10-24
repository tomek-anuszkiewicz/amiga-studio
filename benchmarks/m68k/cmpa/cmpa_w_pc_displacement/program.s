; Compare pc_displacement source with A6; CMPA leaves A6 unchanged.
; helper opcode or extension-plus-helper pair at the same effective offset.
    dc.w $2803
entry:
    rept {{instruction_count}}
    cmpa.w *-2(pc),a6
    move.l d3,d4
    cmpa.w *-2(pc),a6
    move.l d3,d4
    cmpa.w *-2(pc),a6
    move.l d3,d4
    cmpa.w *-2(pc),a6
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
