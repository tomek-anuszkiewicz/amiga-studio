; Load A6 from the selected source EA, then restore it.
; helper opcode or extension-plus-helper pair at the same effective offset.
    dc.w $2c47
entry:
    rept {{instruction_count}}
    movea.w *-10(pc,d6.w),a6
    movea.l d7,a6
    movea.w *-10(pc,d6.w),a6
    movea.l d7,a6
    movea.w *-10(pc,d6.w),a6
    movea.l d7,a6
    movea.w *-10(pc,d6.w),a6
    movea.l d7,a6
    endr
    dc.b "M68K_END_MARKER!"
