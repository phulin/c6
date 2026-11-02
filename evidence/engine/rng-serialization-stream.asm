
/home/phulin/c6-assets/DLC/Expansion2/Binaries/Win64/GameCore_XP2_FinalRelease.dll:     file format pei-x86-64


Disassembly of section .text:

00000001809aa140 <.text+0x9a9140>:
   1809aa140:	40 57                	rex push rdi
   1809aa142:	41 56                	push   r14
   1809aa144:	48 83 ec 28          	sub    rsp,0x28
   1809aa148:	48 89 6c 24 50       	mov    QWORD PTR [rsp+0x50],rbp
   1809aa14d:	49 8b f8             	mov    rdi,r8
   1809aa150:	48 89 74 24 20       	mov    QWORD PTR [rsp+0x20],rsi
   1809aa155:	4c 8b f2             	mov    r14,rdx
   1809aa158:	48 8b 32             	mov    rsi,QWORD PTR [rdx]
   1809aa15b:	48 8b e9             	mov    rbp,rcx
   1809aa15e:	48 83 fe ff          	cmp    rsi,0xffffffffffffffff
   1809aa162:	75 04                	jne    0x1809aa168
   1809aa164:	48 8b 71 10          	mov    rsi,QWORD PTR [rcx+0x10]
   1809aa168:	48 8b 49 18          	mov    rcx,QWORD PTR [rcx+0x18]
   1809aa16c:	48 3b f1             	cmp    rsi,rcx
   1809aa16f:	0f 83 9f 00 00 00    	jae    0x1809aa214
   1809aa175:	48 89 5c 24 48       	mov    QWORD PTR [rsp+0x48],rbx
   1809aa17a:	48 8b 5a 08          	mov    rbx,QWORD PTR [rdx+0x8]
   1809aa17e:	48 8d 04 33          	lea    rax,[rbx+rsi*1]
   1809aa182:	48 3b c1             	cmp    rax,rcx
   1809aa185:	76 06                	jbe    0x1809aa18d
   1809aa187:	48 8b d9             	mov    rbx,rcx
   1809aa18a:	48 2b de             	sub    rbx,rsi
   1809aa18d:	4c 8b 42 10          	mov    r8,QWORD PTR [rdx+0x10]
   1809aa191:	45 84 c9             	test   r9b,r9b
   1809aa194:	75 34                	jne    0x1809aa1ca
   1809aa196:	48 83 3f 00          	cmp    QWORD PTR [rdi],0x0
   1809aa19a:	75 2e                	jne    0x1809aa1ca
   1809aa19c:	4d 85 c0             	test   r8,r8
   1809aa19f:	74 11                	je     0x1809aa1b2
   1809aa1a1:	48 8b 45 08          	mov    rax,QWORD PTR [rbp+0x8]
   1809aa1a5:	33 d2                	xor    edx,edx
   1809aa1a7:	48 03 c6             	add    rax,rsi
   1809aa1aa:	49 f7 f0             	div    r8
   1809aa1ad:	48 85 d2             	test   rdx,rdx
   1809aa1b0:	75 18                	jne    0x1809aa1ca
   1809aa1b2:	48 8b 4d 08          	mov    rcx,QWORD PTR [rbp+0x8]
   1809aa1b6:	48 03 ce             	add    rcx,rsi
   1809aa1b9:	48 89 5f 10          	mov    QWORD PTR [rdi+0x10],rbx
   1809aa1bd:	48 89 0f             	mov    QWORD PTR [rdi],rcx
   1809aa1c0:	48 c7 47 18 00 00 00 	mov    QWORD PTR [rdi+0x18],0x0
   1809aa1c7:	00 
   1809aa1c8:	eb 21                	jmp    0x1809aa1eb
   1809aa1ca:	48 8b d3             	mov    rdx,rbx
   1809aa1cd:	48 8b cf             	mov    rcx,rdi
   1809aa1d0:	e8 ab 78 fe ff       	call   0x180991a80
   1809aa1d5:	84 c0                	test   al,al
   1809aa1d7:	74 16                	je     0x1809aa1ef
   1809aa1d9:	48 8b 55 08          	mov    rdx,QWORD PTR [rbp+0x8]
   1809aa1dd:	4c 8b c3             	mov    r8,rbx
   1809aa1e0:	48 8b 0f             	mov    rcx,QWORD PTR [rdi]
   1809aa1e3:	48 03 d6             	add    rdx,rsi
   1809aa1e6:	e8 15 6b fe ff       	call   0x180990d00
   1809aa1eb:	48 89 5f 08          	mov    QWORD PTR [rdi+0x8],rbx
   1809aa1ef:	48 8d 0c 33          	lea    rcx,[rbx+rsi*1]
   1809aa1f3:	48 8b 5c 24 48       	mov    rbx,QWORD PTR [rsp+0x48]
   1809aa1f8:	48 39 4d 10          	cmp    QWORD PTR [rbp+0x10],rcx
   1809aa1fc:	48 8d 55 10          	lea    rdx,[rbp+0x10]
   1809aa200:	48 8d 44 24 40       	lea    rax,[rsp+0x40]
   1809aa205:	48 89 4c 24 40       	mov    QWORD PTR [rsp+0x40],rcx
   1809aa20a:	48 0f 47 c2          	cmova  rax,rdx
   1809aa20e:	48 8b 00             	mov    rax,QWORD PTR [rax]
   1809aa211:	48 89 02             	mov    QWORD PTR [rdx],rax
   1809aa214:	49 83 7e 18 00       	cmp    QWORD PTR [r14+0x18],0x0
   1809aa219:	48 8b 74 24 20       	mov    rsi,QWORD PTR [rsp+0x20]
   1809aa21e:	48 8b 6c 24 50       	mov    rbp,QWORD PTR [rsp+0x50]
   1809aa223:	75 07                	jne    0x1809aa22c
   1809aa225:	49 83 7e 20 00       	cmp    QWORD PTR [r14+0x20],0x0
   1809aa22a:	74 15                	je     0x1809aa241
   1809aa22c:	49 8b 4e 18          	mov    rcx,QWORD PTR [r14+0x18]
   1809aa230:	48 8b d7             	mov    rdx,rdi
   1809aa233:	49 8b 46 20          	mov    rax,QWORD PTR [r14+0x20]
   1809aa237:	48 83 c4 28          	add    rsp,0x28
   1809aa23b:	41 5e                	pop    r14
   1809aa23d:	5f                   	pop    rdi
   1809aa23e:	48 ff e0             	rex.W jmp rax
   1809aa241:	48 83 c4 28          	add    rsp,0x28
   1809aa245:	41 5e                	pop    r14
   1809aa247:	5f                   	pop    rdi
   1809aa248:	c3                   	ret


/home/phulin/c6-assets/DLC/Expansion2/Binaries/Win64/GameCore_XP2_FinalRelease.dll:     file format pei-x86-64


Disassembly of section .text:

00000001809aa250 <.text+0x9a9250>:
   1809aa250:	48 89 5c 24 08       	mov    QWORD PTR [rsp+0x8],rbx
   1809aa255:	57                   	push   rdi
   1809aa256:	48 83 ec 70          	sub    rsp,0x70
   1809aa25a:	49 8b c0             	mov    rax,r8
   1809aa25d:	48 8b da             	mov    rbx,rdx
   1809aa260:	48 8b f9             	mov    rdi,rcx
   1809aa263:	48 8b d0             	mov    rdx,rax
   1809aa266:	4d 8b c1             	mov    r8,r9
   1809aa269:	48 8d 4c 24 20       	lea    rcx,[rsp+0x20]
   1809aa26e:	e8 2d 77 fe ff       	call   0x1809919a0
   1809aa273:	48 8b 03             	mov    rax,QWORD PTR [rbx]
   1809aa276:	4c 8d 44 24 20       	lea    r8,[rsp+0x20]
   1809aa27b:	48 89 44 24 40       	mov    QWORD PTR [rsp+0x40],rax
   1809aa280:	48 8d 54 24 40       	lea    rdx,[rsp+0x40]
   1809aa285:	48 8b 43 08          	mov    rax,QWORD PTR [rbx+0x8]
   1809aa289:	41 b1 01             	mov    r9b,0x1
   1809aa28c:	48 89 44 24 48       	mov    QWORD PTR [rsp+0x48],rax
   1809aa291:	48 8b cf             	mov    rcx,rdi
   1809aa294:	48 8b 43 20          	mov    rax,QWORD PTR [rbx+0x20]
   1809aa298:	48 89 44 24 60       	mov    QWORD PTR [rsp+0x60],rax
   1809aa29d:	48 8b 43 18          	mov    rax,QWORD PTR [rbx+0x18]
   1809aa2a1:	48 89 44 24 58       	mov    QWORD PTR [rsp+0x58],rax
   1809aa2a6:	48 8b 07             	mov    rax,QWORD PTR [rdi]
   1809aa2a9:	48 c7 44 24 50 00 00 	mov    QWORD PTR [rsp+0x50],0x0
   1809aa2b0:	00 00 
   1809aa2b2:	ff 50 28             	call   QWORD PTR [rax+0x28]
   1809aa2b5:	48 8d 4c 24 20       	lea    rcx,[rsp+0x20]
   1809aa2ba:	e8 21 77 fe ff       	call   0x1809919e0
   1809aa2bf:	48 8b 9c 24 80 00 00 	mov    rbx,QWORD PTR [rsp+0x80]
   1809aa2c6:	00 
   1809aa2c7:	48 83 c4 70          	add    rsp,0x70
   1809aa2cb:	5f                   	pop    rdi
   1809aa2cc:	c3                   	ret


/home/phulin/c6-assets/DLC/Expansion2/Binaries/Win64/GameCore_XP2_FinalRelease.dll:     file format pei-x86-64


Disassembly of section .text:

00000001800de150 <.text+0xdd150>:
   1800de150:	4c 8b dc             	mov    r11,rsp
   1800de153:	49 89 5b 18          	mov    QWORD PTR [r11+0x18],rbx
   1800de157:	49 89 6b 20          	mov    QWORD PTR [r11+0x20],rbp
   1800de15b:	57                   	push   rdi
   1800de15c:	48 83 ec 50          	sub    rsp,0x50
   1800de160:	33 ed                	xor    ebp,ebp
   1800de162:	49 c7 43 c8 ff ff ff 	mov    QWORD PTR [r11-0x38],0xffffffffffffffff
   1800de169:	ff 
   1800de16a:	89 6a 10             	mov    DWORD PTR [rdx+0x10],ebp
   1800de16d:	4d 8d 43 08          	lea    r8,[r11+0x8]
   1800de171:	48 8b 01             	mov    rax,QWORD PTR [rcx]
   1800de174:	48 8b da             	mov    rbx,rdx
   1800de177:	49 8d 53 c8          	lea    rdx,[r11-0x38]
   1800de17b:	89 6c 24 60          	mov    DWORD PTR [rsp+0x60],ebp
   1800de17f:	44 8d 4d 04          	lea    r9d,[rbp+0x4]
   1800de183:	49 c7 43 d0 04 00 00 	mov    QWORD PTR [r11-0x30],0x4
   1800de18a:	00 
   1800de18b:	48 8b f9             	mov    rdi,rcx
   1800de18e:	49 c7 43 d8 10 00 00 	mov    QWORD PTR [r11-0x28],0x10
   1800de195:	00 
   1800de196:	49 89 6b e0          	mov    QWORD PTR [r11-0x20],rbp
   1800de19a:	49 89 6b e8          	mov    QWORD PTR [r11-0x18],rbp
   1800de19e:	ff 50 30             	call   QWORD PTR [rax+0x30]
   1800de1a1:	48 8b 07             	mov    rax,QWORD PTR [rdi]
   1800de1a4:	48 8b cf             	mov    rcx,rdi
   1800de1a7:	ff 50 40             	call   QWORD PTR [rax+0x40]
   1800de1aa:	8b 4c 24 60          	mov    ecx,DWORD PTR [rsp+0x60]
   1800de1ae:	85 c9                	test   ecx,ecx
   1800de1b0:	0f 84 9e 00 00 00    	je     0x1800de254
   1800de1b6:	8b 43 08             	mov    eax,DWORD PTR [rbx+0x8]
   1800de1b9:	8b d1                	mov    edx,ecx
   1800de1bb:	48 89 74 24 68       	mov    QWORD PTR [rsp+0x68],rsi
   1800de1c0:	48 3b c1             	cmp    rax,rcx
   1800de1c3:	73 0f                	jae    0x1800de1d4
   1800de1c5:	41 b0 01             	mov    r8b,0x1
   1800de1c8:	48 8b cb             	mov    rcx,rbx
   1800de1cb:	e8 b0 26 2e 00       	call   0x1803c0880
   1800de1d0:	8b 4c 24 60          	mov    ecx,DWORD PTR [rsp+0x60]
   1800de1d4:	8b f5                	mov    esi,ebp
   1800de1d6:	85 c9                	test   ecx,ecx
   1800de1d8:	74 75                	je     0x1800de24f
   1800de1da:	66 0f 1f 44 00 00    	nop    WORD PTR [rax+rax*1+0x0]
   1800de1e0:	8b 53 08             	mov    edx,DWORD PTR [rbx+0x8]
   1800de1e3:	39 53 10             	cmp    DWORD PTR [rbx+0x10],edx
   1800de1e6:	75 0d                	jne    0x1800de1f5
   1800de1e8:	ff c2                	inc    edx
   1800de1ea:	45 33 c0             	xor    r8d,r8d
   1800de1ed:	48 8b cb             	mov    rcx,rbx
   1800de1f0:	e8 8b 26 2e 00       	call   0x1803c0880
   1800de1f5:	ff 43 10             	inc    DWORD PTR [rbx+0x10]
   1800de1f8:	48 8d 54 24 20       	lea    rdx,[rsp+0x20]
   1800de1fd:	8b 4b 10             	mov    ecx,DWORD PTR [rbx+0x10]
   1800de200:	41 b9 04 00 00 00    	mov    r9d,0x4
   1800de206:	48 8b 03             	mov    rax,QWORD PTR [rbx]
   1800de209:	ff c9                	dec    ecx
   1800de20b:	4c 8b 17             	mov    r10,QWORD PTR [rdi]
   1800de20e:	48 c7 44 24 20 ff ff 	mov    QWORD PTR [rsp+0x20],0xffffffffffffffff
   1800de215:	ff ff 
   1800de217:	48 c7 44 24 28 04 00 	mov    QWORD PTR [rsp+0x28],0x4
   1800de21e:	00 00 
   1800de220:	4c 8d 04 88          	lea    r8,[rax+rcx*4]
   1800de224:	48 c7 44 24 30 10 00 	mov    QWORD PTR [rsp+0x30],0x10
   1800de22b:	00 00 
   1800de22d:	48 8b cf             	mov    rcx,rdi
   1800de230:	48 89 6c 24 38       	mov    QWORD PTR [rsp+0x38],rbp
   1800de235:	48 89 6c 24 40       	mov    QWORD PTR [rsp+0x40],rbp
   1800de23a:	41 ff 52 30          	call   QWORD PTR [r10+0x30]
   1800de23e:	48 8b 07             	mov    rax,QWORD PTR [rdi]
   1800de241:	48 8b cf             	mov    rcx,rdi
   1800de244:	ff 50 40             	call   QWORD PTR [rax+0x40]
   1800de247:	ff c6                	inc    esi
   1800de249:	3b 74 24 60          	cmp    esi,DWORD PTR [rsp+0x60]
   1800de24d:	72 91                	jb     0x1800de1e0
   1800de24f:	48 8b 74 24 68       	mov    rsi,QWORD PTR [rsp+0x68]
   1800de254:	48 8b 5c 24 70       	mov    rbx,QWORD PTR [rsp+0x70]
   1800de259:	48 8b 6c 24 78       	mov    rbp,QWORD PTR [rsp+0x78]
   1800de25e:	48 83 c4 50          	add    rsp,0x50
   1800de262:	5f                   	pop    rdi
   1800de263:	c3                   	ret


/home/phulin/c6-assets/DLC/Expansion2/Binaries/Win64/GameCore_XP2_FinalRelease.dll:     file format pei-x86-64


Disassembly of section .text:

00000001808972d0 <.text+0x8962d0>:
   1808972d0:	c2 00 00             	ret    0x0


/home/phulin/c6-assets/DLC/Expansion2/Binaries/Win64/GameCore_XP2_FinalRelease.dll:     file format pei-x86-64


Disassembly of section .text:

00000001809908f0 <.text+0x98f8f0>:
   1809908f0:	48 ff 25 89 6d 02 00 	rex.W jmp QWORD PTR [rip+0x26d89]        # 0x1809b7680

