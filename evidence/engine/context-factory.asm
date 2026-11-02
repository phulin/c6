
/home/phulin/c6-assets/DLC/Expansion2/Binaries/Win64/GameCore_XP2_FinalRelease.dll:     file format pei-x86-64


Disassembly of section .text:

00000001805907a0 <.text+0x58f7a0>:
   1805907a0:	48 83 ec 28          	sub    rsp,0x28
   1805907a4:	0f 10 0d 1d fa 47 00 	movups xmm1,XMMWORD PTR [rip+0x47fa1d]        # 0x180a101c8
   1805907ab:	4c 8b c9             	mov    r9,rcx
   1805907ae:	0f 10 02             	movups xmm0,XMMWORD PTR [rdx]
   1805907b1:	66 48 0f 7e c8       	movq   rax,xmm1
   1805907b6:	0f 29 4c 24 10       	movaps XMMWORD PTR [rsp+0x10],xmm1
   1805907bb:	66 49 0f 7e c0       	movq   r8,xmm0
   1805907c0:	0f 29 04 24          	movaps XMMWORD PTR [rsp],xmm0
   1805907c4:	4c 3b c0             	cmp    r8,rax
   1805907c7:	75 0c                	jne    0x1805907d5
   1805907c9:	48 8b 44 24 08       	mov    rax,QWORD PTR [rsp+0x8]
   1805907ce:	48 3b 44 24 18       	cmp    rax,QWORD PTR [rsp+0x18]
   1805907d3:	74 2b                	je     0x180590800
   1805907d5:	0f 10 0d fc f9 47 00 	movups xmm1,XMMWORD PTR [rip+0x47f9fc]        # 0x180a101d8
   1805907dc:	66 48 0f 7e c1       	movq   rcx,xmm0
   1805907e1:	0f 29 44 24 10       	movaps XMMWORD PTR [rsp+0x10],xmm0
   1805907e6:	66 48 0f 7e c8       	movq   rax,xmm1
   1805907eb:	0f 29 0c 24          	movaps XMMWORD PTR [rsp],xmm1
   1805907ef:	48 3b c8             	cmp    rcx,rax
   1805907f2:	75 14                	jne    0x180590808
   1805907f4:	48 8b 44 24 18       	mov    rax,QWORD PTR [rsp+0x18]
   1805907f9:	48 3b 44 24 08       	cmp    rax,QWORD PTR [rsp+0x8]
   1805907fe:	75 08                	jne    0x180590808
   180590800:	49 8b c1             	mov    rax,r9
   180590803:	48 83 c4 28          	add    rsp,0x28
   180590807:	c3                   	ret
   180590808:	33 c0                	xor    eax,eax
   18059080a:	48 83 c4 28          	add    rsp,0x28
   18059080e:	c3                   	ret


/home/phulin/c6-assets/DLC/Expansion2/Binaries/Win64/GameCore_XP2_FinalRelease.dll:     file format pei-x86-64


Disassembly of section .text:

000000018058fc60 <.text+0x58ec60>:
   18058fc60:	48 83 ec 28          	sub    rsp,0x28
   18058fc64:	48 8b 0d 35 7a 5d 00 	mov    rcx,QWORD PTR [rip+0x5d7a35]        # 0x180b676a0
   18058fc6b:	48 83 f9 ff          	cmp    rcx,0xffffffffffffffff
   18058fc6f:	75 39                	jne    0x18058fcaa
   18058fc71:	45 33 c0             	xor    r8d,r8d
   18058fc74:	33 d2                	xor    edx,edx
   18058fc76:	33 c9                	xor    ecx,ecx
   18058fc78:	ff 15 c2 75 42 00    	call   QWORD PTR [rip+0x4275c2]        # 0x1809b7240
   18058fc7e:	41 b9 04 00 00 00    	mov    r9d,0x4
   18058fc84:	c7 44 24 30 02 00 00 	mov    DWORD PTR [rsp+0x30],0x2
   18058fc8b:	00 
   18058fc8c:	48 8b c8             	mov    rcx,rax
   18058fc8f:	48 89 05 0a 7a 5d 00 	mov    QWORD PTR [rip+0x5d7a0a],rax        # 0x180b676a0
   18058fc96:	4c 8d 44 24 30       	lea    r8,[rsp+0x30]
   18058fc9b:	33 d2                	xor    edx,edx
   18058fc9d:	ff 15 bd 75 42 00    	call   QWORD PTR [rip+0x4275bd]        # 0x1809b7260
   18058fca3:	48 8b 0d f6 79 5d 00 	mov    rcx,QWORD PTR [rip+0x5d79f6]        # 0x180b676a0
   18058fcaa:	33 d2                	xor    edx,edx
   18058fcac:	44 8d 42 10          	lea    r8d,[rdx+0x10]
   18058fcb0:	ff 15 9a 75 42 00    	call   QWORD PTR [rip+0x42759a]        # 0x1809b7250
   18058fcb6:	48 85 c0             	test   rax,rax
   18058fcb9:	74 08                	je     0x18058fcc3
   18058fcbb:	48 8b c8             	mov    rcx,rax
   18058fcbe:	e8 7d fe ff ff       	call   0x18058fb40
   18058fcc3:	48 8b 15 fe f3 5f 00 	mov    rdx,QWORD PTR [rip+0x5ff3fe]        # 0x180b8f0c8
   18058fcca:	48 3b 15 ff f3 5f 00 	cmp    rdx,QWORD PTR [rip+0x5ff3ff]        # 0x180b8f0d0
   18058fcd1:	48 89 44 24 30       	mov    QWORD PTR [rsp+0x30],rax
   18058fcd6:	73 1b                	jae    0x18058fcf3
   18058fcd8:	48 8b ca             	mov    rcx,rdx
   18058fcdb:	48 83 c2 08          	add    rdx,0x8
   18058fcdf:	48 89 15 e2 f3 5f 00 	mov    QWORD PTR [rip+0x5ff3e2],rdx        # 0x180b8f0c8
   18058fce6:	48 85 c9             	test   rcx,rcx
   18058fce9:	74 1e                	je     0x18058fd09
   18058fceb:	48 89 01             	mov    QWORD PTR [rcx],rax
   18058fcee:	48 83 c4 28          	add    rsp,0x28
   18058fcf2:	c3                   	ret
   18058fcf3:	4c 8d 44 24 30       	lea    r8,[rsp+0x30]
   18058fcf8:	48 8d 0d c1 f3 5f 00 	lea    rcx,[rip+0x5ff3c1]        # 0x180b8f0c0
   18058fcff:	e8 6c 3b a8 ff       	call   0x180013870
   18058fd04:	48 8b 44 24 30       	mov    rax,QWORD PTR [rsp+0x30]
   18058fd09:	48 83 c4 28          	add    rsp,0x28
   18058fd0d:	c3                   	ret


/home/phulin/c6-assets/DLC/Expansion2/Binaries/Win64/GameCore_XP2_FinalRelease.dll:     file format pei-x86-64


Disassembly of section .text:

000000018058fb40 <.text+0x58eb40>:
   18058fb40:	40 53                	rex push rbx
   18058fb42:	48 83 ec 30          	sub    rsp,0x30
   18058fb46:	48 8d 05 9b 06 48 00 	lea    rax,[rip+0x48069b]        # 0x180a101e8
   18058fb4d:	48 c7 41 08 00 00 00 	mov    QWORD PTR [rcx+0x8],0x0
   18058fb54:	00 
   18058fb55:	48 89 01             	mov    QWORD PTR [rcx],rax
   18058fb58:	48 8b d9             	mov    rbx,rcx
   18058fb5b:	e8 c0 3a e0 ff       	call   0x180393620
   18058fb60:	8b d0                	mov    edx,eax
   18058fb62:	c7 44 24 28 00 00 00 	mov    DWORD PTR [rsp+0x28],0x0
   18058fb69:	00 
   18058fb6a:	41 b9 6b 00 00 00    	mov    r9d,0x6b
   18058fb70:	c7 44 24 20 33 00 00 	mov    DWORD PTR [rsp+0x20],0x33
   18058fb77:	00 
   18058fb78:	4c 8d 05 f1 08 48 00 	lea    r8,[rip+0x4808f1]        # 0x180a10470
   18058fb7f:	b9 80 1a 00 00       	mov    ecx,0x1a80
   18058fb84:	e8 77 0d 40 00       	call   0x180990900
   18058fb89:	48 85 c0             	test   rax,rax
   18058fb8c:	74 15                	je     0x18058fba3
   18058fb8e:	48 8b c8             	mov    rcx,rax
   18058fb91:	e8 4a 08 1c 00       	call   0x1807503e0
   18058fb96:	48 89 43 08          	mov    QWORD PTR [rbx+0x8],rax
   18058fb9a:	48 8b c3             	mov    rax,rbx
   18058fb9d:	48 83 c4 30          	add    rsp,0x30
   18058fba1:	5b                   	pop    rbx
   18058fba2:	c3                   	ret
   18058fba3:	48 c7 43 08 00 00 00 	mov    QWORD PTR [rbx+0x8],0x0
   18058fbaa:	00 
   18058fbab:	48 8b c3             	mov    rax,rbx
   18058fbae:	48 83 c4 30          	add    rsp,0x30
   18058fbb2:	5b                   	pop    rbx
   18058fbb3:	c3                   	ret


/home/phulin/c6-assets/DLC/Expansion2/Binaries/Win64/GameCore_XP2_FinalRelease.dll:     file format pei-x86-64


Disassembly of section .text:

00000001807503e0 <.text+0x74f3e0>:
   1807503e0:	40 53                	rex push rbx
   1807503e2:	48 83 ec 20          	sub    rsp,0x20
   1807503e6:	48 8b d9             	mov    rbx,rcx
   1807503e9:	e8 e2 50 8f ff       	call   0x1800454d0
   1807503ee:	48 8d 05 7b 4b 2f 00 	lea    rax,[rip+0x2f4b7b]        # 0x180a44f70
   1807503f5:	48 89 03             	mov    QWORD PTR [rbx],rax
   1807503f8:	48 8b c3             	mov    rax,rbx
   1807503fb:	48 83 c4 20          	add    rsp,0x20
   1807503ff:	5b                   	pop    rbx
   180750400:	c3                   	ret

