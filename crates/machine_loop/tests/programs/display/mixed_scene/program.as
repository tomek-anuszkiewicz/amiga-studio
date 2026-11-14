; Deterministic OCS workload, loaded at $1000. No ROM or operating system.
; Five scrolling planes, eight sprites, raster Copper, ABCD Blitter, four audio
; channels, CIA timer/keyboard interrupts, and continuous CPU Chip RAM work.
; HRM chapters 2-8: ordinary guest register writes and DMA data structures.
CUSTOM      equ $dff000
STATS       equ $8000
COPPER      equ $10000
PLANES      equ $20000
PLANE_SIZE  equ $4000
SPRITES     equ $38000
AUDIO       equ $3e000
SCRATCH     equ $50000
CIAA_SDR    equ $bfec01
CIAA_ICR    equ $bfed01
CIAA_CRA    equ $bfee01
CIAA_CRB    equ $bfef01
DMACON      equ $096
INTENA      equ $09a
INTREQ      equ $09c
BLTSIZE     equ $058

entry:
    move.w #$2700,sr
    move.l #$7f000,sp
    lea CUSTOM,a6
    move.w #$7fff,DMACON(a6)
    move.w #$7fff,INTENA(a6)
    move.w #$7fff,INTREQ(a6)
    move.b #$7f,CIAA_ICR
    move.b #$7f,$bfdd00
    lea STATS,a0
    moveq #15,d0
clear_stats:
    clr.l (a0)+
    dbra d0,clear_stats
    lea $8,a0
    moveq #45,d0
set_vectors:
    move.l #fault,(a0)+
    dbra d0,set_vectors
    move.l #ports_irq,$68
    move.l #vblank_irq,$6c
    move.l #audio_irq,$70

    ; 64-byte row stride leaves space for coarse scrolling beyond 320 pixels.
    lea PLANES,a0
    moveq #4,d6
    move.w #$f0f0,d2
fill_plane:
    move.w #8191,d0
fill_words:
    move.w d2,(a0)+
    ror.w #1,d2
    dbra d0,fill_words
    eori.w #$55aa,d2
    dbra d6,fill_plane

    ; Repeated 16-line emblems make each DMA sprite visible over 96 lines.
    lea SPRITES,a0
    moveq #7,d6
fill_sprite:
    move.l a0,a2
    clr.l (a0)+
    moveq #5,d5
sprite_repeat:
    lea emblem(pc),a1
    moveq #15,d0
sprite_row:
    move.l (a1)+,(a0)+
    dbra d0,sprite_row
    dbra d5,sprite_repeat
    clr.l (a0)
    lea $800(a2),a0
    dbra d6,fill_sprite

    lea AUDIO,a0
    move.w #511,d0
    move.w #$807f,d1
fill_audio:
    move.w d1,(a0)+
    addi.w #$070b,d1
    dbra d0,fill_audio
    lea $0a0(a6),a0
    move.l #AUDIO,d1
    move.w #124,d2
    moveq #3,d0
audio_setup:
    move.l d1,(a0)
    move.w #128,4(a0)
    move.w d2,6(a0)
    move.w #40,8(a0)
    addi.l #256,d1
    addi.w #37,d2
    lea 16(a0),a0
    dbra d0,audio_setup

    lea palette(pc),a0
    lea $180(a6),a1
    moveq #31,d0
set_palette:
    move.w (a0)+,(a1)+
    dbra d0,set_palette
    ; Center 256 lines within the visible PAL border: raster lines 40..295.
    move.w #$2881,$08e(a6)
    move.w #$28c1,$090(a6)
    ; Start after sprite 7 slots; early wide fetch would steal its DMA.
    move.w #$0038,$092(a6)
    move.w #$00d0,$094(a6)
    move.w #$5200,$100(a6)
    move.w #$0024,$104(a6)
    move.w #24,$108(a6)
    move.w #24,$10a(a6)
    move.w #$f0c3,$098(a6)

    ; Header reloads the five planes and eight sprite streams every field.
    lea COPPER,a0
    move.w #$0e0,d0
    move.l #PLANES,d1
    moveq #4,d7
copper_planes:
    bsr copper_pointer
    addi.l #PLANE_SIZE,d1
    dbra d7,copper_planes
    move.w #$120,d0
    move.l #SPRITES,d1
    moveq #7,d7
copper_sprites:
    bsr copper_pointer
    addi.l #$800,d1
    dbra d7,copper_sprites
    ; Cover the full PAL border, including the extra line of a long field.
    ; Change all three colors during HBLANK, before the visible line starts.
    move.w #$1a19,d0
    move.w #285,d7
    moveq #0,d1
copper_raster:
    move.w d0,(a0)+
    move.w #$fffe,(a0)+
    move.w #$180,(a0)+
    move.w d1,(a0)+
    move.w #$182,(a0)+
    move.w d1,d2
    eori.w #$0357,d2
    move.w d2,(a0)+
    move.w #$184,(a0)+
    eori.w #$0246,d2
    move.w d2,(a0)+
    addi.w #$0100,d0
    addi.w #$0011,d1
    andi.w #$0777,d1
    cmpi.w #$0019,d0
    bne.s copper_next_line
    ; Finish line 255 before comparing the wrapped eight-bit vertical counter.
    move.l #$ffdffffe,(a0)+
copper_next_line:
    dbra d7,copper_raster
    move.l #$fffffffe,(a0)
    move.l #COPPER,$080(a6)
    move.w #0,$088(a6)

    move.w #$ffff,$044(a6)
    move.w #$ffff,$046(a6)
    move.w #$0f96,$040(a6)
    move.w #$1000,$042(a6)
    move.w #24,$060(a6)
    move.w #24,$062(a6)
    move.w #24,$064(a6)
    move.w #24,$066(a6)

    ; CIA-A timer B and keyboard serial input share level 2.
    move.b #0,CIAA_CRA
    move.b #0,CIAA_CRB
    move.b #$00,$bfe601
    move.b #$20,$bfe701
    move.b #$11,CIAA_CRB
    move.b #$8a,CIAA_ICR
    ; CIA-B timers run without enabling disk/serial peripheral activity.
    move.b #$71,$bfd400
    move.b #$13,$bfd500
    move.b #$11,$bfde00
    move.b #$a9,$bfd600
    move.b #$27,$bfd700
    move.b #$11,$bfdf00
    bsr acknowledge_key
    bsr update_frame
    move.w #$83ef,DMACON(a6)
    move.w #$c7a8,INTENA(a6)
    move.w #$2000,sr
    lea SCRATCH,a4
    move.l #$12345678,d2
    move.l #$13579bdf,d3
    move.l #$2468ace0,d4
    moveq #17,d5
main:
    addq.l #1,STATS+4
    add.l d2,d3
    eor.l d3,d4
    rol.l #3,d4
    mulu.w d2,d5
    addq.l #1,d2
    move.l d4,(a4)+
    cmpa.l #SCRATCH+$4000,a4
    bcs.s scratch_ready
    lea SCRATCH,a4
scratch_ready:
    move.w $002(a6),d0
    btst #14,d0
    bne.s main
    addq.l #1,STATS+8
    move.l #PLANES,$050(a6)
    move.l #PLANES+PLANE_SIZE,$04c(a6)
    move.l #PLANES+PLANE_SIZE*2,$048(a6)
    move.l #PLANES+PLANE_SIZE*4,$054(a6)
    move.w #$2014,BLTSIZE(a6)
    bra.s main

copper_pointer:
    move.w d0,(a0)+
    swap d1
    move.w d1,(a0)+
    swap d1
    addq.w #2,d0
    move.w d0,(a0)+
    move.w d1,(a0)+
    addq.w #2,d0
    rts

vblank_irq:
    movem.l d0-d7/a0-a3,-(sp)
    move.w #$0020,INTREQ(a6)
    addq.l #1,STATS
    bsr update_frame
    movem.l (sp)+,d0-d7/a0-a3
    rte

update_frame:
    move.w STATS+2,d6
    move.w d6,d0
    andi.w #15,d0
    move.w d0,d1
    lsl.w #4,d1
    or.w d1,d0
    move.w d0,$102(a6)
    ; Coarse plane displacement stays within the 64-byte row padding.
    move.w d6,d0
    lsr.w #3,d0
    andi.w #14,d0
    lea COPPER+6,a0
    moveq #4,d7
scroll_planes:
    move.w d0,(a0)
    addi.w #$4000,d0
    lea 8(a0),a0
    dbra d7,scroll_planes
    lea SPRITES,a0
    moveq #0,d7
move_sprites:
    move.w d7,d4
    cmpi.w #7,d4
    bne.s sprite_position_index
    moveq #6,d4
sprite_position_index:
    move.w d6,d0
    add.w d4,d0
    andi.w #63,d0
    btst #1,d7
    beq.s sprite_forward
    neg.w d0
    addi.w #63,d0
sprite_forward:
    addi.w #76,d0
    move.w d4,d1
    mulu.w #6,d1
    add.w d1,d0
    move.w d4,d2
    andi.w #3,d2
    mulu.w #12,d2
    addi.w #60,d2
    move.w d6,d3
    lsr.w #3,d3
    andi.w #15,d3
    add.w d3,d2
    move.w d2,d3
    lsl.w #8,d2
    or.w d0,d2
    move.w d2,(a0)
    addi.w #96,d3
    lsl.w #8,d3
    ; Last pair is attached; other channels remain independent.
    cmpi.w #7,d7
    bne.s sprite_control
    ori.w #$80,d3
sprite_control:
    move.w d3,2(a0)
    lea $800(a0),a0
    addq.w #1,d7
    cmpi.w #8,d7
    bne.s move_sprites
    move.w $00e(a6),d0
    or.w d0,STATS+24
    move.w $00a(a6),STATS+26
    move.w $00c(a6),STATS+28
    move.b $bfe001,STATS+31
    move.w #$0001,$034(a6)
    rts

ports_irq:
    movem.l d0-d1,-(sp)
    move.b CIAA_ICR,d1
    addq.l #1,STATS+16
    btst #3,d1
    beq.s ports_done
    move.b CIAA_SDR,STATS+30
    addq.l #1,STATS+12
    bsr acknowledge_key
ports_done:
    move.w #$0008,INTREQ(a6)
    movem.l (sp)+,d0-d1
    rte

acknowledge_key:
    ; Allow final serial recovery, then hold KDAT low for over 85 microseconds.
    moveq #31,d0
key_recovery:
    dbra d0,key_recovery
    move.b #$40,CIAA_CRA
    move.w #127,d0
key_pulse:
    dbra d0,key_pulse
    move.b #0,CIAA_CRA
    rts

audio_irq:
    addq.l #1,STATS+20
    move.w #$0780,INTREQ(a6)
    rte

fault:
    move.w #$2700,sr
    move.l #$bad,STATS+36
    move.w #$f00,$dff180
    bra.s fault

palette:
    dc.w $012,$123,$234,$345,$135,$246,$357,$024
    dc.w $132,$243,$354,$465,$153,$264,$375,$486
    dc.w $000,$f42,$ff6,$fff,$000,$2df,$6ff,$fff
    dc.w $000,$d3f,$f9f,$fff,$000,$4f6,$af4,$fff
emblem:
    dc.w $0180,$0000,$03c0,$0180,$07e0,$03c0,$0ff0,$0660
    dc.w $1ff8,$0c30,$3ffc,$1818,$7ffe,$300c,$ffff,$6006
    dc.w $ffff,$6006,$7ffe,$300c,$3ffc,$1818,$1ff8,$0c30
    dc.w $0ff0,$0660,$07e0,$03c0,$03c0,$0180,$0180,$0000
