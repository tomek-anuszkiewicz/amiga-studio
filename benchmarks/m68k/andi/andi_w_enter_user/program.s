; The target enters user mode. A setup privilege trap restores supervisor mode.
restore_supervisor:
    move.w #$2700,(a7)
    addq.l #4,2(a7)
    rte
entry:
    rept {{instruction_count}}
    andi.w #$dfff,sr
    move.l d7,d0
    move.w #$2700,sr
    andi.w #$dfff,sr
    move.l d7,d0
    move.w #$2700,sr
    andi.w #$dfff,sr
    move.l d7,d0
    move.w #$2700,sr
    andi.w #$dfff,sr
    move.l d7,d0
    move.w #$2700,sr
    endr
    dc.b "M68K_END_MARKER!"
