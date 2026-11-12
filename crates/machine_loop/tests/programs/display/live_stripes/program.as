; HRM chapter 3: one low-resolution bitplane, repeated row using BPL1MOD.
; Load/entry $1000, supervisor mode, interrupts masked, low-memory overlay off.
; No firmware, media, Copper list or host-supplied display state is required.
; Twenty $F0F0 words at $2000 produce four-pixel stripes across 320 pixels.
; COLOR01 alternates green/red once per field; COLOR00 stays dark blue.
entry:
    move.w #$2700,sr
    lea $dff000,a0
    move.w #$7fff,$096(a0)
    move.w #$7fff,$09a(a0)
    lea $2000,a1
    moveq #19,d1
fill_row:
    move.w #$f0f0,(a1)+
    dbra d1,fill_row
    move.l #$2000,$0e0(a0)
    move.w #$2c81,$08e(a0)
    move.w #$f4c1,$090(a0)
    move.w #$0038,$092(a0)
    move.w #$00d0,$094(a0)
    move.w #$1000,$100(a0)
    move.w #0,$102(a0)
    move.w #-40,$108(a0)
    move.w #$0002,$180(a0)
    move.w #$00f0,d0
    move.w d0,$182(a0)
    move.w #$8300,$096(a0)
wait_bottom:
    cmp.b #$e0,$006(a0)
    bne.s wait_bottom
wait_top:
    cmp.b #$20,$006(a0)
    bne.s wait_top
    eori.w #$0ff0,d0
    move.w d0,$182(a0)
    bra.s wait_bottom
