; Restore SR after each target to keep its supervisor and condition-code input fixed.
entry:
    rept {{instruction_count}}
    ori.w #$0011,sr
    move.w #$2704,sr
    ori.w #$0011,sr
    move.w #$2704,sr
    ori.w #$0011,sr
    move.w #$2704,sr
    ori.w #$0011,sr
    move.w #$2704,sr
    endr
    dc.b "M68K_END_MARKER!"
