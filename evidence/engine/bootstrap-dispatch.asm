
Messages::GameTurnComplete::Execute

/home/phulin/c6-assets/DLC/Expansion2/Binaries/Win64/GameCore_XP2_FinalRelease.dll:     file format pei-x86-64


Disassembly of section .text:

0000000180035f70 <.text+0x34f70>:
   180035f70:	40 53                	rex push rbx
   180035f72:	48 83 ec 20          	sub    rsp,0x20
   180035f76:	48 8b d9             	mov    rbx,rcx
   180035f79:	e8 12 4c 01 00       	call   0x18004ab90
   180035f7e:	84 c0                	test   al,al
   180035f80:	75 4b                	jne    0x180035fcd
   180035f82:	8b 4b 18             	mov    ecx,DWORD PTR [rbx+0x18]
   180035f85:	83 f9 40             	cmp    ecx,0x40
   180035f88:	73 43                	jae    0x180035fcd
   180035f8a:	85 c9                	test   ecx,ecx
   180035f8c:	78 3f                	js     0x180035fcd
   180035f8e:	e8 fd ed 00 00       	call   0x180044d90
   180035f93:	48 8b d8             	mov    rbx,rax
   180035f96:	48 85 c0             	test   rax,rax
   180035f99:	74 32                	je     0x180035fcd
   180035f9b:	b2 01                	mov    dl,0x1
   180035f9d:	48 8b c8             	mov    rcx,rax
   180035fa0:	e8 8b 7b 02 00       	call   0x18005db30
   180035fa5:	e8 a6 b5 91 00       	call   0x180951550
   180035faa:	48 8b c8             	mov    rcx,rax
   180035fad:	e8 2e 83 60 00       	call   0x18063e2e0
   180035fb2:	80 bb 88 02 00 00 00 	cmp    BYTE PTR [rbx+0x288],0x0
   180035fb9:	74 12                	je     0x180035fcd
   180035fbb:	e8 90 b5 91 00       	call   0x180951550
   180035fc0:	48 8b c8             	mov    rcx,rax
   180035fc3:	48 83 c4 20          	add    rsp,0x20
   180035fc7:	5b                   	pop    rbx
   180035fc8:	e9 13 83 60 00       	jmp    0x18063e2e0
   180035fcd:	48                   	rex.W

Messages::Manager::ProcessMessages

/home/phulin/c6-assets/DLC/Expansion2/Binaries/Win64/GameCore_XP2_FinalRelease.dll:     file format pei-x86-64


Disassembly of section .text:

00000001800372f0 <.text+0x362f0>:
   1800372f0:	48 89 5c 24 08       	mov    QWORD PTR [rsp+0x8],rbx
   1800372f5:	48 89 74 24 10       	mov    QWORD PTR [rsp+0x10],rsi
   1800372fa:	57                   	push   rdi
   1800372fb:	48 83 ec 60          	sub    rsp,0x60
   1800372ff:	33 f6                	xor    esi,esi
   180037301:	48 8d 3d e0 36 b5 00 	lea    rdi,[rip+0xb536e0]        # 0x180b8a9e8
   180037308:	0f 1f 84 00 00 00 00 	nop    DWORD PTR [rax+rax*1+0x0]
   18003730f:	00 
   180037310:	48 8b 1d d1 36 b5 00 	mov    rbx,QWORD PTR [rip+0xb536d1]        # 0x180b8a9e8
   180037317:	48 8b ce             	mov    rcx,rsi
   18003731a:	48 8b c3             	mov    rax,rbx
   18003731d:	48 3b df             	cmp    rbx,rdi
   180037320:	74 0b                	je     0x18003732d
   180037322:	48 8b 00             	mov    rax,QWORD PTR [rax]
   180037325:	48 ff c1             	inc    rcx
   180037328:	48 3b c7             	cmp    rax,rdi
   18003732b:	75 f5                	jne    0x180037322
   18003732d:	85 c9                	test   ecx,ecx
   18003732f:	0f 84 92 00 00 00    	je     0x1800373c7
   180037335:	44 8b 43 20          	mov    r8d,DWORD PTR [rbx+0x20]
   180037339:	48 8d 4c 24 30       	lea    rcx,[rsp+0x30]
   18003733e:	44 2b 43 18          	sub    r8d,DWORD PTR [rbx+0x18]
   180037342:	48 8b 53 18          	mov    rdx,QWORD PTR [rbx+0x18]
   180037346:	e8 75 2b 97 00       	call   0x1809a9ec0
   18003734b:	8b 4b 10             	mov    ecx,DWORD PTR [rbx+0x10]
   18003734e:	48 8d 54 24 30       	lea    rdx,[rsp+0x30]
   180037353:	e8 f8 07 00 00       	call   0x180037b50
   180037358:	48 8b 1d 89 36 b5 00 	mov    rbx,QWORD PTR [rip+0xb53689]        # 0x180b8a9e8
   18003735f:	48 8b 4b 08          	mov    rcx,QWORD PTR [rbx+0x8]
   180037363:	48 8b 03             	mov    rax,QWORD PTR [rbx]
   180037366:	48 89 01             	mov    QWORD PTR [rcx],rax
   180037369:	48 8b 0b             	mov    rcx,QWORD PTR [rbx]
   18003736c:	48 8b 43 08          	mov    rax,QWORD PTR [rbx+0x8]
   180037370:	48 89 41 08          	mov    QWORD PTR [rcx+0x8],rax
   180037374:	48 8b 53 18          	mov    rdx,QWORD PTR [rbx+0x18]
   180037378:	48 85 d2             	test   rdx,rdx
   18003737b:	74 10                	je     0x18003738d
   18003737d:	4c 8b 43 28          	mov    r8,QWORD PTR [rbx+0x28]
   180037381:	48 8d 4b 30          	lea    rcx,[rbx+0x30]
   180037385:	4c 2b c2             	sub    r8,rdx
   180037388:	e8 93 fa ff ff       	call   0x180036e20
   18003738d:	41 b8 38 00 00 00    	mov    r8d,0x38
   180037393:	48 8d 0d 5e 36 b5 00 	lea    rcx,[rip+0xb5365e]        # 0x180b8a9f8
   18003739a:	48 8b d3             	mov    rdx,rbx
   18003739d:	e8 7e fa ff ff       	call   0x180036e20
   1800373a2:	c7 44 24 20 22 c2 a4 	mov    DWORD PTR [rsp+0x20],0x9ea4c222
   1800373a9:	9e 
   1800373aa:	89 74 24 24          	mov    DWORD PTR [rsp+0x24],esi
   1800373ae:	48 8b 4c 24 20       	mov    rcx,QWORD PTR [rsp+0x20]
   1800373b3:	e8 f8 35 5c 00       	call   0x1805fa9b0
   1800373b8:	48 8d 4c 24 30       	lea    rcx,[rsp+0x30]
   1800373bd:	e8 be 2b 97 00       	call   0x1809a9f80
   1800373c2:	e9 49 ff ff ff       	jmp    0x180037310
   1800373c7:	48 8b 5c 24 70       	mov    rbx,QWORD PTR [rsp+0x70]
   1800373cc:	48 8b 74 24 78       	mov    rsi,QWORD PTR [rsp+0x78]
   1800373d1:	48 83 c4 60          	add    rsp,0x60
   1800373d5:	5f                   	pop    rdi
   1800373d6:	c3                   	ret

Messages::PlayerTurnComplete::Execute

/home/phulin/c6-assets/DLC/Expansion2/Binaries/Win64/GameCore_XP2_FinalRelease.dll:     file format pei-x86-64


Disassembly of section .text:

0000000180039920 <.text+0x38920>:
   180039920:	40 55                	rex push rbp
   180039922:	41 56                	push   r14
   180039924:	48 8d ac 24 58 fe ff 	lea    rbp,[rsp-0x1a8]
   18003992b:	ff 
   18003992c:	48 81 ec a8 02 00 00 	sub    rsp,0x2a8
   180039933:	4c 8b f1             	mov    r14,rcx
   180039936:	8b 49 18             	mov    ecx,DWORD PTR [rcx+0x18]
   180039939:	83 f9 40             	cmp    ecx,0x40
   18003993c:	0f 83 a1 04 00 00    	jae    0x180039de3
   180039942:	48 89 9c 24 d0 02 00 	mov    QWORD PTR [rsp+0x2d0],rbx
   180039949:	00 
   18003994a:	e8 41 b4 00 00       	call   0x180044d90
   18003994f:	48 8b d8             	mov    rbx,rax
   180039952:	48 85 c0             	test   rax,rax
   180039955:	0f 84 80 04 00 00    	je     0x180039ddb
   18003995b:	48 89 b4 24 d8 02 00 	mov    QWORD PTR [rsp+0x2d8],rsi
   180039962:	00 
   180039963:	48 89 bc 24 a0 02 00 	mov    QWORD PTR [rsp+0x2a0],rdi
   18003996a:	00 
   18003996b:	4c 89 a4 24 98 02 00 	mov    QWORD PTR [rsp+0x298],r12
   180039972:	00 
   180039973:	4c 89 bc 24 90 02 00 	mov    QWORD PTR [rsp+0x290],r15
   18003997a:	00 
   18003997b:	e8 d0 7b 91 00       	call   0x180951550
   180039980:	48 8b cb             	mov    rcx,rbx
   180039983:	48 8b f8             	mov    rdi,rax
   180039986:	e8 95 fe 2b 00       	call   0x1802f9820
   18003998b:	84 c0                	test   al,al
   18003998d:	0f 84 ff 01 00 00    	je     0x180039b92
   180039993:	80 bf 20 02 00 00 00 	cmp    BYTE PTR [rdi+0x220],0x0
   18003999a:	0f 84 f2 01 00 00    	je     0x180039b92
   1800399a0:	48 8d 95 c8 01 00 00 	lea    rdx,[rbp+0x1c8]
   1800399a7:	48 c7 85 c0 01 00 00 	mov    QWORD PTR [rbp+0x1c0],0x1
   1800399ae:	01 00 00 00 
   1800399b2:	48 8d 8d c0 01 00 00 	lea    rcx,[rbp+0x1c0]
   1800399b9:	48 c7 85 c8 01 00 00 	mov    QWORD PTR [rbp+0x1c8],0x1
   1800399c0:	01 00 00 00 
   1800399c4:	e8 b7 7e 95 00       	call   0x180991880
   1800399c9:	45 33 e4             	xor    r12d,r12d
   1800399cc:	4c 39 a5 c0 01 00 00 	cmp    QWORD PTR [rbp+0x1c0],r12
   1800399d3:	0f 84 6f 01 00 00    	je     0x180039b48
   1800399d9:	e8 32 7d 95 00       	call   0x180991710
   1800399de:	48 8b d0             	mov    rdx,rax
   1800399e1:	48 8d 4c 24 50       	lea    rcx,[rsp+0x50]
   1800399e6:	e8 15 7e 95 00       	call   0x180991800
   1800399eb:	48 8d 0d 0e 53 98 00 	lea    rcx,[rip+0x98530e]        # 0x1809bed00
   1800399f2:	e8 99 7b 95 00       	call   0x180991590
   1800399f7:	8b d8                	mov    ebx,eax
   1800399f9:	83 f8 ff             	cmp    eax,0xffffffff
   1800399fc:	0f 84 70 01 00 00    	je     0x180039b72
   180039a02:	8b 4c 24 64          	mov    ecx,DWORD PTR [rsp+0x64]
   180039a06:	4c 8d 05 0b 49 98 00 	lea    r8,[rip+0x98490b]        # 0x1809be318
   180039a0d:	44 8b 4c 24 50       	mov    r9d,DWORD PTR [rsp+0x50]
   180039a12:	41 8d 54 24 17       	lea    edx,[r12+0x17]
   180039a17:	89 4c 24 40          	mov    DWORD PTR [rsp+0x40],ecx
   180039a1b:	8b 4c 24 60          	mov    ecx,DWORD PTR [rsp+0x60]
   180039a1f:	89 4c 24 38          	mov    DWORD PTR [rsp+0x38],ecx
   180039a23:	8b 4c 24 5c          	mov    ecx,DWORD PTR [rsp+0x5c]
   180039a27:	89 4c 24 30          	mov    DWORD PTR [rsp+0x30],ecx
   180039a2b:	8b 4c 24 58          	mov    ecx,DWORD PTR [rsp+0x58]
   180039a2f:	89 4c 24 28          	mov    DWORD PTR [rsp+0x28],ecx
   180039a33:	8b 4c 24 54          	mov    ecx,DWORD PTR [rsp+0x54]
   180039a37:	89 4c 24 20          	mov    DWORD PTR [rsp+0x20],ecx
   180039a3b:	48 8d 4c 24 70       	lea    rcx,[rsp+0x70]
   180039a40:	e8 8b 79 95 00       	call   0x1809913d0
   180039a45:	8b f8                	mov    edi,eax
   180039a47:	44 8d 3c 18          	lea    r15d,[rax+rbx*1]
   180039a4b:	bb 17 01 00 00       	mov    ebx,0x117
   180039a50:	44 3b fb             	cmp    r15d,ebx
   180039a53:	0f 86 b4 00 00 00    	jbe    0x180039b0d
   180039a59:	41 8d 47 01          	lea    eax,[r15+0x1]
   180039a5d:	44 89 64 24 20       	mov    DWORD PTR [rsp+0x20],r12d
   180039a62:	8b c8                	mov    ecx,eax
   180039a64:	45 8d 4c 24 03       	lea    r9d,[r12+0x3]
   180039a69:	45 8d 44 24 4a       	lea    r8d,[r12+0x4a]
   180039a6e:	8b d8                	mov    ebx,eax
   180039a70:	48 8d 15 c9 52 98 00 	lea    rdx,[rip+0x9852c9]        # 0x1809bed40
   180039a77:	e8 84 6e 95 00       	call   0x180990900
   180039a7c:	45 8d 4c 24 03       	lea    r9d,[r12+0x3]
   180039a81:	44 89 64 24 20       	mov    DWORD PTR [rsp+0x20],r12d
   180039a86:	45 8d 44 24 4a       	lea    r8d,[r12+0x4a]
   180039a8b:	8b cb                	mov    ecx,ebx
   180039a8d:	48 8d 15 ac 52 98 00 	lea    rdx,[rip+0x9852ac]        # 0x1809bed40
   180039a94:	48 8b f0             	mov    rsi,rax
   180039a97:	e8 64 6e 95 00       	call   0x180990900
   180039a9c:	44 8b c3             	mov    r8d,ebx
   180039a9f:	33 d2                	xor    edx,edx
   180039aa1:	48 8b ce             	mov    rcx,rsi
   180039aa4:	48 8b f8             	mov    rdi,rax
   180039aa7:	e8 b4 72 95 00       	call   0x180990d60
   180039aac:	44 8b c3             	mov    r8d,ebx
   180039aaf:	33 d2                	xor    edx,edx
   180039ab1:	48 8b cf             	mov    rcx,rdi
   180039ab4:	e8 a7 72 95 00       	call   0x180990d60
   180039ab9:	4c 8d 05 40 52 98 00 	lea    r8,[rip+0x985240]        # 0x1809bed00
   180039ac0:	41 8b d7             	mov    edx,r15d
   180039ac3:	48 8b ce             	mov    rcx,rsi
   180039ac6:	e8 05 79 95 00       	call   0x1809913d0
   180039acb:	45 8b c7             	mov    r8d,r15d
   180039ace:	48 8d 54 24 70       	lea    rdx,[rsp+0x70]
   180039ad3:	48 8b cf             	mov    rcx,rdi
   180039ad6:	e8 e5 79 95 00       	call   0x1809914c0
   180039adb:	45 8b c7             	mov    r8d,r15d
   180039ade:	48 8b d6             	mov    rdx,rsi
   180039ae1:	48 8b cf             	mov    rcx,rdi
   180039ae4:	e8 d7 79 95 00       	call   0x1809914c0
   180039ae9:	45 8b c7             	mov    r8d,r15d
   180039aec:	48 8d 0d 4d 48 98 00 	lea    rcx,[rip+0x98484d]        # 0x1809be340
   180039af3:	48 8b d7             	mov    rdx,rdi
   180039af6:	e8 e5 7d 95 00       	call   0x1809918e0
   180039afb:	48 8b ce             	mov    rcx,rsi
   180039afe:	e8 ed 6d 95 00       	call   0x1809908f0
   180039b03:	48 8b cf             	mov    rcx,rdi
   180039b06:	e8 e5 6d 95 00       	call   0x1809908f0
   180039b0b:	eb 65                	jmp    0x180039b72
   180039b0d:	4c 8d 05 ec 51 98 00 	lea    r8,[rip+0x9851ec]        # 0x1809bed00
   180039b14:	ba 00 01 00 00       	mov    edx,0x100
   180039b19:	48 8d 8d 90 00 00 00 	lea    rcx,[rbp+0x90]
   180039b20:	e8 ab 78 95 00       	call   0x1809913d0
   180039b25:	44 8b c3             	mov    r8d,ebx
   180039b28:	48 8d 95 90 00 00 00 	lea    rdx,[rbp+0x90]
   180039b2f:	44 2b c7             	sub    r8d,edi
   180039b32:	48 8d 4c 24 70       	lea    rcx,[rsp+0x70]
   180039b37:	e8 84 79 95 00       	call   0x1809914c0
   180039b3c:	44 3b fb             	cmp    r15d,ebx
   180039b3f:	41 0f 42 df          	cmovb  ebx,r15d
   180039b43:	44 8b c3             	mov    r8d,ebx
   180039b46:	eb 19                	jmp    0x180039b61
   180039b48:	4c 8d 05 b1 51 98 00 	lea    r8,[rip+0x9851b1]        # 0x1809bed00
   180039b4f:	ba 17 01 00 00       	mov    edx,0x117
   180039b54:	48 8d 4c 24 70       	lea    rcx,[rsp+0x70]
   180039b59:	e8 72 78 95 00       	call   0x1809913d0
   180039b5e:	44 8b c0             	mov    r8d,eax
   180039b61:	48 8d 54 24 70       	lea    rdx,[rsp+0x70]
   180039b66:	48 8d 0d d3 47 98 00 	lea    rcx,[rip+0x9847d3]        # 0x1809be340
   180039b6d:	e8 6e 7d 95 00       	call   0x1809918e0
   180039b72:	41 8b 56 18          	mov    edx,DWORD PTR [r14+0x18]
   180039b76:	c7 44 24 50 61 52 b8 	mov    DWORD PTR [rsp+0x50],0xa7b85261
   180039b7d:	a7 
   180039b7e:	44 89 64 24 54       	mov    DWORD PTR [rsp+0x54],r12d
   180039b83:	48 8b 4c 24 50       	mov    rcx,QWORD PTR [rsp+0x50]
   180039b88:	e8 63 29 5c 00       	call   0x1805fc4f0
   180039b8d:	e9 29 02 00 00       	jmp    0x180039dbb
   180039b92:	48 8b cb             	mov    rcx,rbx
   180039b95:	e8 06 37 02 00       	call   0x18005d2a0
   180039b9a:	84 c0                	test   al,al
   180039b9c:	0f 84 e8 01 00 00    	je     0x180039d8a
   180039ba2:	48 8d 95 c8 01 00 00 	lea    rdx,[rbp+0x1c8]
   180039ba9:	48 c7 85 c0 01 00 00 	mov    QWORD PTR [rbp+0x1c0],0x1
   180039bb0:	01 00 00 00 
   180039bb4:	48 8d 8d c0 01 00 00 	lea    rcx,[rbp+0x1c0]
   180039bbb:	48 c7 85 c8 01 00 00 	mov    QWORD PTR [rbp+0x1c8],0x1
   180039bc2:	01 00 00 00 
   180039bc6:	e8 b5 7c 95 00       	call   0x180991880
   180039bcb:	48 83 bd c0 01 00 00 	cmp    QWORD PTR [rbp+0x1c0],0x0
   180039bd2:	00 
   180039bd3:	0f 84 81 01 00 00    	je     0x180039d5a
   180039bd9:	e8 32 7b 95 00       	call   0x180991710
   180039bde:	48 8b d0             	mov    rdx,rax
   180039be1:	48 8d 4c 24 50       	lea    rcx,[rsp+0x50]
   180039be6:	e8 15 7c 95 00       	call   0x180991800
   180039beb:	41 8b 56 18          	mov    edx,DWORD PTR [r14+0x18]
   180039bef:	48 8d 0d ba 51 98 00 	lea    rcx,[rip+0x9851ba]        # 0x1809bedb0
   180039bf6:	e8 95 79 95 00       	call   0x180991590
   180039bfb:	8b d8                	mov    ebx,eax
   180039bfd:	83 f8 ff             	cmp    eax,0xffffffff
   180039c00:	0f 84 b5 01 00 00    	je     0x180039dbb
   180039c06:	8b 4c 24 64          	mov    ecx,DWORD PTR [rsp+0x64]
   180039c0a:	4c 8d 05 07 47 98 00 	lea    r8,[rip+0x984707]        # 0x1809be318
   180039c11:	44 8b 4c 24 50       	mov    r9d,DWORD PTR [rsp+0x50]
   180039c16:	ba 17 00 00 00       	mov    edx,0x17
   180039c1b:	89 4c 24 40          	mov    DWORD PTR [rsp+0x40],ecx
   180039c1f:	8b 4c 24 60          	mov    ecx,DWORD PTR [rsp+0x60]
   180039c23:	89 4c 24 38          	mov    DWORD PTR [rsp+0x38],ecx
   180039c27:	8b 4c 24 5c          	mov    ecx,DWORD PTR [rsp+0x5c]
   180039c2b:	89 4c 24 30          	mov    DWORD PTR [rsp+0x30],ecx
   180039c2f:	8b 4c 24 58          	mov    ecx,DWORD PTR [rsp+0x58]
   180039c33:	89 4c 24 28          	mov    DWORD PTR [rsp+0x28],ecx
   180039c37:	8b 4c 24 54          	mov    ecx,DWORD PTR [rsp+0x54]
   180039c3b:	89 4c 24 20          	mov    DWORD PTR [rsp+0x20],ecx
   180039c3f:	48 8d 4c 24 70       	lea    rcx,[rsp+0x70]
   180039c44:	e8 87 77 95 00       	call   0x1809913d0
   180039c49:	8b f8                	mov    edi,eax
   180039c4b:	44 8d 3c 18          	lea    r15d,[rax+rbx*1]
   180039c4f:	bb 17 01 00 00       	mov    ebx,0x117
   180039c54:	44 3b fb             	cmp    r15d,ebx
   180039c57:	0f 86 be 00 00 00    	jbe    0x180039d1b
   180039c5d:	45 33 e4             	xor    r12d,r12d
   180039c60:	41 8d 47 01          	lea    eax,[r15+0x1]
   180039c64:	48 8d 15 d5 50 98 00 	lea    rdx,[rip+0x9850d5]        # 0x1809bed40
   180039c6b:	8b c8                	mov    ecx,eax
   180039c6d:	8b d8                	mov    ebx,eax
   180039c6f:	44 89 64 24 20       	mov    DWORD PTR [rsp+0x20],r12d
   180039c74:	45 8d 4c 24 03       	lea    r9d,[r12+0x3]
   180039c79:	45 8d 44 24 52       	lea    r8d,[r12+0x52]
   180039c7e:	e8 7d 6c 95 00       	call   0x180990900
   180039c83:	45 8d 4c 24 03       	lea    r9d,[r12+0x3]
   180039c88:	44 89 64 24 20       	mov    DWORD PTR [rsp+0x20],r12d
   180039c8d:	45 8d 44 24 52       	lea    r8d,[r12+0x52]
   180039c92:	8b cb                	mov    ecx,ebx
   180039c94:	48 8d 15 a5 50 98 00 	lea    rdx,[rip+0x9850a5]        # 0x1809bed40
   180039c9b:	48 8b f0             	mov    rsi,rax
   180039c9e:	e8 5d 6c 95 00       	call   0x180990900
   180039ca3:	44 8b c3             	mov    r8d,ebx
   180039ca6:	33 d2                	xor    edx,edx
   180039ca8:	48 8b ce             	mov    rcx,rsi
   180039cab:	48 8b f8             	mov    rdi,rax
   180039cae:	e8 ad 70 95 00       	call   0x180990d60
   180039cb3:	44 8b c3             	mov    r8d,ebx
   180039cb6:	33 d2                	xor    edx,edx
   180039cb8:	48 8b cf             	mov    rcx,rdi
   180039cbb:	e8 a0 70 95 00       	call   0x180990d60
   180039cc0:	45 8b 4e 18          	mov    r9d,DWORD PTR [r14+0x18]
   180039cc4:	4c 8d 05 e5 50 98 00 	lea    r8,[rip+0x9850e5]        # 0x1809bedb0
   180039ccb:	41 8b d7             	mov    edx,r15d
   180039cce:	48 8b ce             	mov    rcx,rsi
   180039cd1:	e8 fa 76 95 00       	call   0x1809913d0
   180039cd6:	45 8b c7             	mov    r8d,r15d
   180039cd9:	48 8d 54 24 70       	lea    rdx,[rsp+0x70]
   180039cde:	48 8b cf             	mov    rcx,rdi
   180039ce1:	e8 da 77 95 00       	call   0x1809914c0
   180039ce6:	45 8b c7             	mov    r8d,r15d
   180039ce9:	48 8b d6             	mov    rdx,rsi
   180039cec:	48 8b cf             	mov    rcx,rdi
   180039cef:	e8 cc 77 95 00       	call   0x1809914c0
   180039cf4:	45 8b c7             	mov    r8d,r15d
   180039cf7:	48 8d 0d 42 46 98 00 	lea    rcx,[rip+0x984642]        # 0x1809be340
   180039cfe:	48 8b d7             	mov    rdx,rdi
   180039d01:	e8 da 7b 95 00       	call   0x1809918e0
   180039d06:	48 8b ce             	mov    rcx,rsi
   180039d09:	e8 e2 6b 95 00       	call   0x1809908f0
   180039d0e:	48 8b cf             	mov    rcx,rdi
   180039d11:	e8 da 6b 95 00       	call   0x1809908f0
   180039d16:	e9 a0 00 00 00       	jmp    0x180039dbb
   180039d1b:	45 8b 4e 18          	mov    r9d,DWORD PTR [r14+0x18]
   180039d1f:	4c 8d 05 8a 50 98 00 	lea    r8,[rip+0x98508a]        # 0x1809bedb0
   180039d26:	ba 00 01 00 00       	mov    edx,0x100
   180039d2b:	48 8d 8d 90 00 00 00 	lea    rcx,[rbp+0x90]
   180039d32:	e8 99 76 95 00       	call   0x1809913d0
   180039d37:	44 8b c3             	mov    r8d,ebx
   180039d3a:	48 8d 95 90 00 00 00 	lea    rdx,[rbp+0x90]
   180039d41:	44 2b c7             	sub    r8d,edi
   180039d44:	48 8d 4c 24 70       	lea    rcx,[rsp+0x70]
   180039d49:	e8 72 77 95 00       	call   0x1809914c0
   180039d4e:	44 3b fb             	cmp    r15d,ebx
   180039d51:	41 0f 42 df          	cmovb  ebx,r15d
   180039d55:	44 8b c3             	mov    r8d,ebx
   180039d58:	eb 1d                	jmp    0x180039d77
   180039d5a:	45 8b 4e 18          	mov    r9d,DWORD PTR [r14+0x18]
   180039d5e:	4c 8d 05 4b 50 98 00 	lea    r8,[rip+0x98504b]        # 0x1809bedb0
   180039d65:	ba 17 01 00 00       	mov    edx,0x117
   180039d6a:	48 8d 4c 24 70       	lea    rcx,[rsp+0x70]
   180039d6f:	e8 5c 76 95 00       	call   0x1809913d0
   180039d74:	44 8b c0             	mov    r8d,eax
   180039d77:	48 8d 54 24 70       	lea    rdx,[rsp+0x70]
   180039d7c:	48 8d 0d bd 45 98 00 	lea    rcx,[rip+0x9845bd]        # 0x1809be340
   180039d83:	e8 58 7b 95 00       	call   0x1809918e0
   180039d88:	eb 31                	jmp    0x180039dbb
   180039d8a:	45 8b 46 1c          	mov    r8d,DWORD PTR [r14+0x1c]
   180039d8e:	b2 01                	mov    dl,0x1
   180039d90:	48 8b cb             	mov    rcx,rbx
   180039d93:	e8 98 44 02 00       	call   0x18005e230
   180039d98:	e8 b3 77 91 00       	call   0x180951550
   180039d9d:	48 8b c8             	mov    rcx,rax
   180039da0:	e8 3b 45 60 00       	call   0x18063e2e0
   180039da5:	80 bb 88 02 00 00 00 	cmp    BYTE PTR [rbx+0x288],0x0
   180039dac:	74 0d                	je     0x180039dbb
   180039dae:	e8 9d 77 91 00       	call   0x180951550
   180039db3:	48 8b c8             	mov    rcx,rax
   180039db6:	e8 25 45 60 00       	call   0x18063e2e0
   180039dbb:	4c 8b a4 24 98 02 00 	mov    r12,QWORD PTR [rsp+0x298]
   180039dc2:	00 
   180039dc3:	48 8b bc 24 a0 02 00 	mov    rdi,QWORD PTR [rsp+0x2a0]
   180039dca:	00 
   180039dcb:	48 8b b4 24 d8 02 00 	mov    rsi,QWORD PTR [rsp+0x2d8]
   180039dd2:	00 
   180039dd3:	4c 8b bc 24 90 02 00 	mov    r15,QWORD PTR [rsp+0x290]
   180039dda:	00 
   180039ddb:	48                   	rex.W
   180039ddc:	8b                   	.byte 0x8b
   180039ddd:	9c                   	pushf
   180039dde:	24 d0                	and    al,0xd0

Messages::UnitOperation::Execute

/home/phulin/c6-assets/DLC/Expansion2/Binaries/Win64/GameCore_XP2_FinalRelease.dll:     file format pei-x86-64


Disassembly of section .text:

000000018003aad0 <.text+0x39ad0>:
   18003aad0:	48 89 5c 24 10       	mov    QWORD PTR [rsp+0x10],rbx
   18003aad5:	57                   	push   rdi
   18003aad6:	48 83 ec 20          	sub    rsp,0x20
   18003aada:	0f bf 59 18          	movsx  ebx,WORD PTR [rcx+0x18]
   18003aade:	48 8b f9             	mov    rdi,rcx
   18003aae1:	e8 9a c0 2c 00       	call   0x180306b80
   18003aae6:	83 fb 41             	cmp    ebx,0x41
   18003aae9:	73 67                	jae    0x18003ab52
   18003aaeb:	48 8b 80 48 2b 00 00 	mov    rax,QWORD PTR [rax+0x2b48]
   18003aaf2:	83 3c 98 ff          	cmp    DWORD PTR [rax+rbx*4],0xffffffff
   18003aaf6:	74 5a                	je     0x18003ab52
   18003aaf8:	48 0f bf 5f 18       	movsx  rbx,WORD PTR [rdi+0x18]
   18003aafd:	48 89 74 24 30       	mov    QWORD PTR [rsp+0x30],rsi
   18003ab02:	e8 79 c0 2c 00       	call   0x180306b80
   18003ab07:	8b 57 1c             	mov    edx,DWORD PTR [rdi+0x1c]
   18003ab0a:	48 8b 40 20          	mov    rax,QWORD PTR [rax+0x20]
   18003ab0e:	48 8b 1c d8          	mov    rbx,QWORD PTR [rax+rbx*8]
   18003ab12:	48 8b 8b c8 06 00 00 	mov    rcx,QWORD PTR [rbx+0x6c8]
   18003ab19:	e8 42 98 28 00       	call   0x1802c4360
   18003ab1e:	48 8b f0             	mov    rsi,rax
   18003ab21:	48 85 c0             	test   rax,rax
   18003ab24:	74 27                	je     0x18003ab4d
   18003ab26:	e8 c5 5d 00 00       	call   0x1800408f0
   18003ab2b:	44 8b 4f 48          	mov    r9d,DWORD PTR [rdi+0x48]
   18003ab2f:	4c 8d 47 20          	lea    r8,[rdi+0x20]
   18003ab33:	48 8b d6             	mov    rdx,rsi
   18003ab36:	48 8b c8             	mov    rcx,rax
   18003ab39:	4c 8b 10             	mov    r10,QWORD PTR [rax]
   18003ab3c:	41 ff 52 60          	call   QWORD PTR [r10+0x60]
   18003ab40:	e8 1b c3 14 00       	call   0x180186e60
   18003ab45:	48 8b cb             	mov    rcx,rbx
   18003ab48:	e8 c3 38 02 00       	call   0x18005e410
   18003ab4d:	48 8b 74 24 30       	mov    rsi,QWORD PTR [rsp+0x30]
   18003ab52:	48 8b 5c 24 38       	mov    rbx,QWORD PTR [rsp+0x38]
   18003ab57:	48 83 c4 20          	add    rsp,0x20
   18003ab5b:	5f                   	pop    rdi
   18003ab5c:	c3                   	ret

Context::Globals::SetEngineUtility

/home/phulin/c6-assets/DLC/Expansion2/Binaries/Win64/GameCore_XP2_FinalRelease.dll:     file format pei-x86-64


Disassembly of section .text:

0000000180045050 <.text+0x44050>:
   180045050:	40 53                	rex push rbx
   180045052:	48 83 ec 30          	sub    rsp,0x30
   180045056:	48 8b d9             	mov    rbx,rcx
   180045059:	48 8b 0d 20 5a b4 00 	mov    rcx,QWORD PTR [rip+0xb45a20]        # 0x180b8aa80
   180045060:	48 85 c9             	test   rcx,rcx
   180045063:	74 06                	je     0x18004506b
   180045065:	48 8b 01             	mov    rax,QWORD PTR [rcx]
   180045068:	ff 50 08             	call   QWORD PTR [rax+0x8]
   18004506b:	48 85 db             	test   rbx,rbx
   18004506e:	74 62                	je     0x1800450d2
   180045070:	0f 10 05 31 ab 97 00 	movups xmm0,XMMWORD PTR [rip+0x97ab31]        # 0x1809bfba8
   180045077:	48 8b 03             	mov    rax,QWORD PTR [rbx]
   18004507a:	48 8d 54 24 20       	lea    rdx,[rsp+0x20]
   18004507f:	48 8b cb             	mov    rcx,rbx
   180045082:	0f 29 44 24 20       	movaps XMMWORD PTR [rsp+0x20],xmm0
   180045087:	ff 10                	call   QWORD PTR [rax]
   180045089:	48 89 05 f0 59 b4 00 	mov    QWORD PTR [rip+0xb459f0],rax        # 0x180b8aa80
   180045090:	48 85 c0             	test   rax,rax
   180045093:	74 48                	je     0x1800450dd
   180045095:	48 8b 10             	mov    rdx,QWORD PTR [rax]
   180045098:	48 8b c8             	mov    rcx,rax
   18004509b:	ff 52 10             	call   QWORD PTR [rdx+0x10]
   18004509e:	48 8b 0d db 59 b4 00 	mov    rcx,QWORD PTR [rip+0xb459db]        # 0x180b8aa80
   1800450a5:	48 89 05 dc 59 b4 00 	mov    QWORD PTR [rip+0xb459dc],rax        # 0x180b8aa88
   1800450ac:	48 85 c9             	test   rcx,rcx
   1800450af:	74 37                	je     0x1800450e8
   1800450b1:	48 8b 01             	mov    rax,QWORD PTR [rcx]
   1800450b4:	ff 50 18             	call   QWORD PTR [rax+0x18]
   1800450b7:	48 83 3d c1 59 b4 00 	cmp    QWORD PTR [rip+0xb459c1],0x0        # 0x180b8aa80
   1800450be:	00 
   1800450bf:	48 89 05 ca 59 b4 00 	mov    QWORD PTR [rip+0xb459ca],rax        # 0x180b8aa90
   1800450c6:	74 2b                	je     0x1800450f3
   1800450c8:	48 83 c4 30          	add    rsp,0x30
   1800450cc:	5b                   	pop    rbx
   1800450cd:	e9 2e c3 15 00       	jmp    0x1801a1400
   1800450d2:	48 c7 05 a3 59 b4 00 	mov    QWORD PTR [rip+0xb459a3],0x0        # 0x180b8aa80
   1800450d9:	00 00 00 00 
   1800450dd:	48 c7 05 a0 59 b4 00 	mov    QWORD PTR [rip+0xb459a0],0x0        # 0x180b8aa88
   1800450e4:	00 00 00 00 
   1800450e8:	48 c7 05 9d 59 b4 00 	mov    QWORD PTR [rip+0xb4599d],0x0        # 0x180b8aa90
   1800450ef:	00 00 00 00 
   1800450f3:	48 83 c4 30          	add    rsp,0x30
   1800450f7:	5b                   	pop    rbx
   1800450f8:	c3                   	ret

Context::Instance::Initialize

/home/phulin/c6-assets/DLC/Expansion2/Binaries/Win64/GameCore_XP2_FinalRelease.dll:     file format pei-x86-64


Disassembly of section .text:

0000000180045d90 <.text+0x44d90>:
   180045d90:	40 53                	rex push rbx
   180045d92:	48 83 ec 30          	sub    rsp,0x30
   180045d96:	80 79 40 00          	cmp    BYTE PTR [rcx+0x40],0x0
   180045d9a:	48 8b d9             	mov    rbx,rcx
   180045d9d:	0f 85 21 01 00 00    	jne    0x180045ec4
   180045da3:	48 8b 0d e6 4c b4 00 	mov    rcx,QWORD PTR [rip+0xb44ce6]        # 0x180b8aa90
   180045daa:	48 89 74 24 40       	mov    QWORD PTR [rsp+0x40],rsi
   180045daf:	48 89 7c 24 48       	mov    QWORD PTR [rsp+0x48],rdi
   180045db4:	33 ff                	xor    edi,edi
   180045db6:	48 85 c9             	test   rcx,rcx
   180045db9:	74 1b                	je     0x180045dd6
   180045dbb:	48 8b 01             	mov    rax,QWORD PTR [rcx]
   180045dbe:	44 8d 4f 05          	lea    r9d,[rdi+0x5]
   180045dc2:	4c 8d 05 c7 9e 97 00 	lea    r8,[rip+0x979ec7]        # 0x1809bfc90
   180045dc9:	48 89 7c 24 20       	mov    QWORD PTR [rsp+0x20],rdi
   180045dce:	ba 42 85 31 2a       	mov    edx,0x2a318542
   180045dd3:	ff 50 40             	call   QWORD PTR [rax+0x40]
   180045dd6:	48 8b 0d a3 4c b4 00 	mov    rcx,QWORD PTR [rip+0xb44ca3]        # 0x180b8aa80
   180045ddd:	48 89 1d 7c 4c b4 00 	mov    QWORD PTR [rip+0xb44c7c],rbx        # 0x180b8aa60
   180045de4:	48 8b 01             	mov    rax,QWORD PTR [rcx]
   180045de7:	ff 90 40 01 00 00    	call   QWORD PTR [rax+0x140]
   180045ded:	b9 10 00 00 00       	mov    ecx,0x10
   180045df2:	48 8b f0             	mov    rsi,rax
   180045df5:	e8 06 ab 94 00       	call   0x180990900
   180045dfa:	48 85 c0             	test   rax,rax
   180045dfd:	74 11                	je     0x180045e10
   180045dff:	48 8d 15 aa 9e 97 00 	lea    rdx,[rip+0x979eaa]        # 0x1809bfcb0
   180045e06:	48 8b c8             	mov    rcx,rax
   180045e09:	e8 d2 d6 59 00       	call   0x1805e34e0
   180045e0e:	eb 03                	jmp    0x180045e13
   180045e10:	48 8b c7             	mov    rax,rdi
   180045e13:	4c 8b c0             	mov    r8,rax
   180045e16:	ba 41 00 00 00       	mov    edx,0x41
   180045e1b:	48 8b ce             	mov    rcx,rsi
   180045e1e:	e8 8f 79 95 00       	call   0x18099d7b2
   180045e23:	48 8b 4b 50          	mov    rcx,QWORD PTR [rbx+0x50]
   180045e27:	48 89 43 50          	mov    QWORD PTR [rbx+0x50],rax
   180045e2b:	48 85 c9             	test   rcx,rcx
   180045e2e:	74 06                	je     0x180045e36
   180045e30:	48 8b 01             	mov    rax,QWORD PTR [rcx]
   180045e33:	ff 50 08             	call   QWORD PTR [rax+0x8]
   180045e36:	48 8b 0d 43 4c b4 00 	mov    rcx,QWORD PTR [rip+0xb44c43]        # 0x180b8aa80
   180045e3d:	48 8b 01             	mov    rax,QWORD PTR [rcx]
   180045e40:	ff 90 38 01 00 00    	call   QWORD PTR [rax+0x138]
   180045e46:	b9 10 00 00 00       	mov    ecx,0x10
   180045e4b:	48 8b f0             	mov    rsi,rax
   180045e4e:	e8 ad aa 94 00       	call   0x180990900
   180045e53:	48 85 c0             	test   rax,rax
   180045e56:	74 12                	je     0x180045e6a
   180045e58:	48 8d 15 61 9e 97 00 	lea    rdx,[rip+0x979e61]        # 0x1809bfcc0
   180045e5f:	48 8b c8             	mov    rcx,rax
   180045e62:	e8 79 d6 59 00       	call   0x1805e34e0
   180045e67:	48 8b f8             	mov    rdi,rax
   180045e6a:	4c 8b c7             	mov    r8,rdi
   180045e6d:	ba 41 00 00 00       	mov    edx,0x41
   180045e72:	48 8b ce             	mov    rcx,rsi
   180045e75:	e8 38 79 95 00       	call   0x18099d7b2
   180045e7a:	48 8b 4b 58          	mov    rcx,QWORD PTR [rbx+0x58]
   180045e7e:	48 8b 7c 24 48       	mov    rdi,QWORD PTR [rsp+0x48]
   180045e83:	48 8b 74 24 40       	mov    rsi,QWORD PTR [rsp+0x40]
   180045e88:	48 89 43 58          	mov    QWORD PTR [rbx+0x58],rax
   180045e8c:	48 85 c9             	test   rcx,rcx
   180045e8f:	74 06                	je     0x180045e97
   180045e91:	48 8b 01             	mov    rax,QWORD PTR [rcx]
   180045e94:	ff 50 08             	call   QWORD PTR [rax+0x8]
   180045e97:	48 8d 83 80 0d 00 00 	lea    rax,[rbx+0xd80]
   180045e9e:	48 8b cb             	mov    rcx,rbx
   180045ea1:	48 89 05 d0 4b b4 00 	mov    QWORD PTR [rip+0xb44bd0],rax        # 0x180b8aa78
   180045ea8:	48 8b 03             	mov    rax,QWORD PTR [rbx]
   180045eab:	ff 90 a8 00 00 00    	call   QWORD PTR [rax+0xa8]
   180045eb1:	48 8b c8             	mov    rcx,rax
   180045eb4:	48 89 83 58 1a 00 00 	mov    QWORD PTR [rbx+0x1a58],rax
   180045ebb:	e8 90 7b 5b 00       	call   0x1805fda50
   180045ec0:	c6 43 40 01          	mov    BYTE PTR [rbx+0x40],0x1
   180045ec4:	b0 01                	mov    al,0x1
   180045ec6:	48 83 c4 30          	add    rsp,0x30
   180045eca:	5b                   	pop    rbx
   180045ecb:	c3                   	ret

Context::Instance::InitializeDatabase

/home/phulin/c6-assets/DLC/Expansion2/Binaries/Win64/GameCore_XP2_FinalRelease.dll:     file format pei-x86-64


Disassembly of section .text:

0000000180045ed0 <.text+0x44ed0>:
   180045ed0:	48 83 ec 48          	sub    rsp,0x48
   180045ed4:	48 8d 4c 24 20       	lea    rcx,[rsp+0x20]
   180045ed9:	e8 b2 43 8e 00       	call   0x18092a290
   180045ede:	48 8d 4c 24 20       	lea    rcx,[rsp+0x20]
   180045ee3:	e8 48 37 93 00       	call   0x180979630
   180045ee8:	48 8b 4c 24 38       	mov    rcx,QWORD PTR [rsp+0x38]
   180045eed:	48 85 c9             	test   rcx,rcx
   180045ef0:	74 06                	je     0x180045ef8
   180045ef2:	48 8b 01             	mov    rax,QWORD PTR [rcx]
   180045ef5:	ff 50 08             	call   QWORD PTR [rax+0x8]
   180045ef8:	48 8b 4c 24 30       	mov    rcx,QWORD PTR [rsp+0x30]
   180045efd:	48 85 c9             	test   rcx,rcx
   180045f00:	74 06                	je     0x180045f08
   180045f02:	48 8b 01             	mov    rax,QWORD PTR [rcx]
   180045f05:	ff 50 08             	call   QWORD PTR [rax+0x8]
   180045f08:	48 8b 4c 24 28       	mov    rcx,QWORD PTR [rsp+0x28]
   180045f0d:	48 85 c9             	test   rcx,rcx
   180045f10:	74 06                	je     0x180045f18
   180045f12:	48 8b 01             	mov    rax,QWORD PTR [rcx]
   180045f15:	ff 50 08             	call   QWORD PTR [rax+0x8]
   180045f18:	b0 01                	mov    al,0x1
   180045f1a:	48 83 c4 48          	add    rsp,0x48
   180045f1e:	c3                   	ret

Context::Instance::InitializeNewGame

/home/phulin/c6-assets/DLC/Expansion2/Binaries/Win64/GameCore_XP2_FinalRelease.dll:     file format pei-x86-64


Disassembly of section .text:

0000000180045f80 <.text+0x44f80>:
   180045f80:	48 89 5c 24 18       	mov    QWORD PTR [rsp+0x18],rbx
   180045f85:	57                   	push   rdi
   180045f86:	48 83 ec 20          	sub    rsp,0x20
   180045f8a:	48 89 6c 24 30       	mov    QWORD PTR [rsp+0x30],rbp
   180045f8f:	49 8b e8             	mov    rbp,r8
   180045f92:	48 8b f9             	mov    rdi,rcx
   180045f95:	48 85 d2             	test   rdx,rdx
   180045f98:	74 30                	je     0x180045fca
   180045f9a:	48 81 c1 80 0d 00 00 	add    rcx,0xd80
   180045fa1:	48 89 74 24 38       	mov    QWORD PTR [rsp+0x38],rsi
   180045fa6:	e8 45 e2 11 00       	call   0x1801641f0
   180045fab:	33 db                	xor    ebx,ebx
   180045fad:	0f 1f 00             	nop    DWORD PTR [rax]
   180045fb0:	8b d3                	mov    edx,ebx
   180045fb2:	48 8d 8f 80 0d 00 00 	lea    rcx,[rdi+0xd80]
   180045fb9:	e8 a2 ed 11 00       	call   0x180164d60
   180045fbe:	ff c3                	inc    ebx
   180045fc0:	83 fb 40             	cmp    ebx,0x40
   180045fc3:	7c eb                	jl     0x180045fb0
   180045fc5:	48 8b 74 24 38       	mov    rsi,QWORD PTR [rsp+0x38]
   180045fca:	48 8b 8f 58 1a 00 00 	mov    rcx,QWORD PTR [rdi+0x1a58]
   180045fd1:	48 85 c9             	test   rcx,rcx
   180045fd4:	74 05                	je     0x180045fdb
   180045fd6:	e8 f5 12 85 00       	call   0x1808972d0
   180045fdb:	48 8b 4f 60          	mov    rcx,QWORD PTR [rdi+0x60]
   180045fdf:	48 8d 97 80 0d 00 00 	lea    rdx,[rdi+0xd80]
   180045fe6:	4c 8b c5             	mov    r8,rbp
   180045fe9:	e8 a2 1d 1e 00       	call   0x180227d90
   180045fee:	48 8b 6c 24 30       	mov    rbp,QWORD PTR [rsp+0x30]
   180045ff3:	0f b6 d8             	movzx  ebx,al
   180045ff6:	84 c0                	test   al,al
   180045ff8:	74 1f                	je     0x180046019
   180045ffa:	48 8b 4f 48          	mov    rcx,QWORD PTR [rdi+0x48]
   180045ffe:	48 85 c9             	test   rcx,rcx
   180046001:	74 05                	je     0x180046008
   180046003:	e8 18 78 13 00       	call   0x18017d820
   180046008:	48 8b 8f 60 1a 00 00 	mov    rcx,QWORD PTR [rdi+0x1a60]
   18004600f:	48 85 c9             	test   rcx,rcx
   180046012:	74 05                	je     0x180046019
   180046014:	e8 b7 12 85 00       	call   0x1808972d0
   180046019:	0f b6 c3             	movzx  eax,bl
   18004601c:	48 8b 5c 24 40       	mov    rbx,QWORD PTR [rsp+0x40]
   180046021:	48 83 c4 20          	add    rsp,0x20
   180046025:	5f                   	pop    rdi
   180046026:	c3                   	ret

Context::Instance::StartupGame

/home/phulin/c6-assets/DLC/Expansion2/Binaries/Win64/GameCore_XP2_FinalRelease.dll:     file format pei-x86-64


Disassembly of section .text:

0000000180046be0 <.text+0x45be0>:
   180046be0:	48 89 5c 24 08       	mov    QWORD PTR [rsp+0x8],rbx
   180046be5:	48 89 74 24 10       	mov    QWORD PTR [rsp+0x10],rsi
   180046bea:	57                   	push   rdi
   180046beb:	48 83 ec 30          	sub    rsp,0x30
   180046bef:	48 8b d9             	mov    rbx,rcx
   180046bf2:	e8 29 ca 34 00       	call   0x180393620
   180046bf7:	33 ff                	xor    edi,edi
   180046bf9:	4c 8d 05 f0 90 97 00 	lea    r8,[rip+0x9790f0]        # 0x1809bfcf0
   180046c00:	89 7c 24 28          	mov    DWORD PTR [rsp+0x28],edi
   180046c04:	8b d0                	mov    edx,eax
   180046c06:	41 b9 11 02 00 00    	mov    r9d,0x211
   180046c0c:	c7 44 24 20 33 00 00 	mov    DWORD PTR [rsp+0x20],0x33
   180046c13:	00 
   180046c14:	b9 80 2e 00 00       	mov    ecx,0x2e80
   180046c19:	e8 e2 9c 94 00       	call   0x180990900
   180046c1e:	48 85 c0             	test   rax,rax
   180046c21:	74 0a                	je     0x180046c2d
   180046c23:	48 8b c8             	mov    rcx,rax
   180046c26:	e8 25 cf 75 00       	call   0x1807a3b50
   180046c2b:	eb 03                	jmp    0x180046c30
   180046c2d:	48 8b c7             	mov    rax,rdi
   180046c30:	48 8b 73 68          	mov    rsi,QWORD PTR [rbx+0x68]
   180046c34:	48 89 43 68          	mov    QWORD PTR [rbx+0x68],rax
   180046c38:	48 85 f6             	test   rsi,rsi
   180046c3b:	74 15                	je     0x180046c52
   180046c3d:	48 8b ce             	mov    rcx,rsi
   180046c40:	e8 cb 67 76 00       	call   0x1807ad410
   180046c45:	ba 80 2e 00 00       	mov    edx,0x2e80
   180046c4a:	48 8b ce             	mov    rcx,rsi
   180046c4d:	e8 56 92 96 00       	call   0x1809afea8
   180046c52:	e8 c9 c9 34 00       	call   0x180393620
   180046c57:	8b d0                	mov    edx,eax
   180046c59:	89 7c 24 28          	mov    DWORD PTR [rsp+0x28],edi
   180046c5d:	41 b9 12 02 00 00    	mov    r9d,0x212
   180046c63:	c7 44 24 20 33 00 00 	mov    DWORD PTR [rsp+0x20],0x33
   180046c6a:	00 
   180046c6b:	4c 8d 05 7e 90 97 00 	lea    r8,[rip+0x97907e]        # 0x1809bfcf0
   180046c72:	b9 28 00 00 00       	mov    ecx,0x28
   180046c77:	e8 84 9c 94 00       	call   0x180990900
   180046c7c:	48 85 c0             	test   rax,rax
   180046c7f:	74 0e                	je     0x180046c8f
   180046c81:	48 8b 53 58          	mov    rdx,QWORD PTR [rbx+0x58]
   180046c85:	48 8b c8             	mov    rcx,rax
   180046c88:	e8 63 14 00 00       	call   0x1800480f0
   180046c8d:	eb 03                	jmp    0x180046c92
   180046c8f:	48 8b c7             	mov    rax,rdi
   180046c92:	48 8b 53 70          	mov    rdx,QWORD PTR [rbx+0x70]
   180046c96:	48 8d 4b 70          	lea    rcx,[rbx+0x70]
   180046c9a:	48 89 01             	mov    QWORD PTR [rcx],rax
   180046c9d:	48 85 d2             	test   rdx,rdx
   180046ca0:	74 05                	je     0x180046ca7
   180046ca2:	e8 49 ec ff ff       	call   0x1800458f0
   180046ca7:	e8 74 c9 34 00       	call   0x180393620
   180046cac:	8b d0                	mov    edx,eax
   180046cae:	89 7c 24 28          	mov    DWORD PTR [rsp+0x28],edi
   180046cb2:	41 b9 13 02 00 00    	mov    r9d,0x213
   180046cb8:	c7 44 24 20 33 00 00 	mov    DWORD PTR [rsp+0x20],0x33
   180046cbf:	00 
   180046cc0:	4c 8d 05 29 90 97 00 	lea    r8,[rip+0x979029]        # 0x1809bfcf0
   180046cc7:	b9 90 00 00 00       	mov    ecx,0x90
   180046ccc:	e8 2f 9c 94 00       	call   0x180990900
   180046cd1:	48 85 c0             	test   rax,rax
   180046cd4:	74 0f                	je     0x180046ce5
   180046cd6:	48 8b 53 58          	mov    rdx,QWORD PTR [rbx+0x58]
   180046cda:	48 8b c8             	mov    rcx,rax
   180046cdd:	e8 1e d2 5b 00       	call   0x180603f00
   180046ce2:	48 8b f8             	mov    rdi,rax
   180046ce5:	48 8b 73 78          	mov    rsi,QWORD PTR [rbx+0x78]
   180046ce9:	48 89 7b 78          	mov    QWORD PTR [rbx+0x78],rdi
   180046ced:	48 85 f6             	test   rsi,rsi
   180046cf0:	74 15                	je     0x180046d07
   180046cf2:	48 8b ce             	mov    rcx,rsi
   180046cf5:	e8 56 ea ff ff       	call   0x180045750
   180046cfa:	ba 90 00 00 00       	mov    edx,0x90
   180046cff:	48 8b ce             	mov    rcx,rsi
   180046d02:	e8 a1 91 96 00       	call   0x1809afea8
   180046d07:	48 8b 53 58          	mov    rdx,QWORD PTR [rbx+0x58]
   180046d0b:	48 8b 4b 68          	mov    rcx,QWORD PTR [rbx+0x68]
   180046d0f:	e8 5c b0 78 00       	call   0x1807d1d70
   180046d14:	e8 27 df 59 00       	call   0x1805e4c40
   180046d19:	48 83 7b 60 00       	cmp    QWORD PTR [rbx+0x60],0x0
   180046d1e:	0f 85 b2 00 00 00    	jne    0x180046dd6
   180046d24:	48 8b 03             	mov    rax,QWORD PTR [rbx]
   180046d27:	48 8b cb             	mov    rcx,rbx
   180046d2a:	ff 90 98 00 00 00    	call   QWORD PTR [rax+0x98]
   180046d30:	48 89 43 60          	mov    QWORD PTR [rbx+0x60],rax
   180046d34:	48 89 05 2d 3d b4 00 	mov    QWORD PTR [rip+0xb43d2d],rax        # 0x180b8aa68
   180046d3b:	e8 30 e8 01 00       	call   0x180065570
   180046d40:	48 8b 4b 60          	mov    rcx,QWORD PTR [rbx+0x60]
   180046d44:	48 8b 01             	mov    rax,QWORD PTR [rcx]
   180046d47:	ff 50 18             	call   QWORD PTR [rax+0x18]
   180046d4a:	48 8b 4b 60          	mov    rcx,QWORD PTR [rbx+0x60]
   180046d4e:	48 8b 01             	mov    rax,QWORD PTR [rcx]
   180046d51:	ff 50 20             	call   QWORD PTR [rax+0x20]
   180046d54:	48 8b 43 60          	mov    rax,QWORD PTR [rbx+0x60]
   180046d58:	48 8b 88 78 02 00 00 	mov    rcx,QWORD PTR [rax+0x278]
   180046d5f:	48 89 0d 0a 3d b4 00 	mov    QWORD PTR [rip+0xb43d0a],rcx        # 0x180b8aa70
   180046d66:	48 83 bb 60 1a 00 00 	cmp    QWORD PTR [rbx+0x1a60],0x0
   180046d6d:	00 
   180046d6e:	75 13                	jne    0x180046d83
   180046d70:	48 8b 03             	mov    rax,QWORD PTR [rbx]
   180046d73:	48 8b cb             	mov    rcx,rbx
   180046d76:	ff 90 b0 00 00 00    	call   QWORD PTR [rax+0xb0]
   180046d7c:	48 89 83 60 1a 00 00 	mov    QWORD PTR [rbx+0x1a60],rax
   180046d83:	48 8b 8b 60 1a 00 00 	mov    rcx,QWORD PTR [rbx+0x1a60]
   180046d8a:	e8 a1 47 5c 00       	call   0x18060b530
   180046d8f:	48 8b cb             	mov    rcx,rbx
   180046d92:	e8 e9 f8 ff ff       	call   0x180046680
   180046d97:	48 8b 03             	mov    rax,QWORD PTR [rbx]
   180046d9a:	48 8b cb             	mov    rcx,rbx
   180046d9d:	ff 90 a0 00 00 00    	call   QWORD PTR [rax+0xa0]
   180046da3:	48 89 83 18 1a 00 00 	mov    QWORD PTR [rbx+0x1a18],rax
   180046daa:	48 8b cb             	mov    rcx,rbx
   180046dad:	48 8b 03             	mov    rax,QWORD PTR [rbx]
   180046db0:	ff 90 b8 00 00 00    	call   QWORD PTR [rax+0xb8]
   180046db6:	48 89 83 68 1a 00 00 	mov    QWORD PTR [rbx+0x1a68],rax
   180046dbd:	48 8b d3             	mov    rdx,rbx
   180046dc0:	48 8b c8             	mov    rcx,rax
   180046dc3:	4c 8b 00             	mov    r8,QWORD PTR [rax]
   180046dc6:	41 ff 50 08          	call   QWORD PTR [r8+0x8]
   180046dca:	48 8d 8b 20 1a 00 00 	lea    rcx,[rbx+0x1a20]
   180046dd1:	e8 5a e9 5b 00       	call   0x180605730
   180046dd6:	48 8b 5c 24 40       	mov    rbx,QWORD PTR [rsp+0x40]
   180046ddb:	48 8b 74 24 48       	mov    rsi,QWORD PTR [rsp+0x48]
   180046de0:	48 83 c4 30          	add    rsp,0x30
   180046de4:	5f                   	pop    rdi
   180046de5:	c3                   	ret

Game::Processor::EndTurn

/home/phulin/c6-assets/DLC/Expansion2/Binaries/Win64/GameCore_XP2_FinalRelease.dll:     file format pei-x86-64


Disassembly of section .text:

00000001800498f0 <.text+0x488f0>:
   1800498f0:	40 53                	rex push rbx
   1800498f2:	55                   	push   rbp
   1800498f3:	57                   	push   rdi
   1800498f4:	48 81 ec 90 02 00 00 	sub    rsp,0x290
   1800498fb:	c6 05 1f 12 b4 00 01 	mov    BYTE PTR [rip+0xb4121f],0x1        # 0x180b8ab21
   180049902:	c6 05 17 12 b4 00 01 	mov    BYTE PTR [rip+0xb41217],0x1        # 0x180b8ab20
   180049909:	e8 b2 32 01 00       	call   0x18005cbc0
   18004990e:	e8 ed 19 01 00       	call   0x18005b300
   180049913:	48 8d 94 24 b8 02 00 	lea    rdx,[rsp+0x2b8]
   18004991a:	00 
   18004991b:	48 c7 84 24 b0 02 00 	mov    QWORD PTR [rsp+0x2b0],0x1
   180049922:	00 01 00 00 00 
   180049927:	48 8d 8c 24 b0 02 00 	lea    rcx,[rsp+0x2b0]
   18004992e:	00 
   18004992f:	48 c7 84 24 b8 02 00 	mov    QWORD PTR [rsp+0x2b8],0x1
   180049936:	00 01 00 00 00 
   18004993b:	e8 40 7f 94 00       	call   0x180991880
   180049940:	48 83 bc 24 b0 02 00 	cmp    QWORD PTR [rsp+0x2b0],0x0
   180049947:	00 00 
   180049949:	0f 84 b1 01 00 00    	je     0x180049b00
   18004994f:	e8 bc 7d 94 00       	call   0x180991710
   180049954:	48 8b d0             	mov    rdx,rax
   180049957:	48 8d 4c 24 50       	lea    rcx,[rsp+0x50]
   18004995c:	e8 9f 7e 94 00       	call   0x180991800
   180049961:	48 8b 05 00 11 b4 00 	mov    rax,QWORD PTR [rip+0xb41100]        # 0x180b8aa68
   180049968:	48 8d 0d c1 67 97 00 	lea    rcx,[rip+0x9767c1]        # 0x1809c0130
   18004996f:	8b 90 c0 00 00 00    	mov    edx,DWORD PTR [rax+0xc0]
   180049975:	e8 16 7c 94 00       	call   0x180991590
   18004997a:	8b d8                	mov    ebx,eax
   18004997c:	83 f8 ff             	cmp    eax,0xffffffff
   18004997f:	0f 84 b3 01 00 00    	je     0x180049b38
   180049985:	8b 4c 24 64          	mov    ecx,DWORD PTR [rsp+0x64]
   180049989:	4c 8d 05 88 49 97 00 	lea    r8,[rip+0x974988]        # 0x1809be318
   180049990:	44 8b 4c 24 50       	mov    r9d,DWORD PTR [rsp+0x50]
   180049995:	ba 17 00 00 00       	mov    edx,0x17
   18004999a:	89 4c 24 40          	mov    DWORD PTR [rsp+0x40],ecx
   18004999e:	8b 4c 24 60          	mov    ecx,DWORD PTR [rsp+0x60]
   1800499a2:	89 4c 24 38          	mov    DWORD PTR [rsp+0x38],ecx
   1800499a6:	8b 4c 24 5c          	mov    ecx,DWORD PTR [rsp+0x5c]
   1800499aa:	89 4c 24 30          	mov    DWORD PTR [rsp+0x30],ecx
   1800499ae:	8b 4c 24 58          	mov    ecx,DWORD PTR [rsp+0x58]
   1800499b2:	89 4c 24 28          	mov    DWORD PTR [rsp+0x28],ecx
   1800499b6:	8b 4c 24 54          	mov    ecx,DWORD PTR [rsp+0x54]
   1800499ba:	89 4c 24 20          	mov    DWORD PTR [rsp+0x20],ecx
   1800499be:	48 8d 4c 24 70       	lea    rcx,[rsp+0x70]
   1800499c3:	e8 08 7a 94 00       	call   0x1809913d0
   1800499c8:	8b f8                	mov    edi,eax
   1800499ca:	8d 2c 18             	lea    ebp,[rax+rbx*1]
   1800499cd:	bb 17 01 00 00       	mov    ebx,0x117
   1800499d2:	3b eb                	cmp    ebp,ebx
   1800499d4:	0f 86 dd 00 00 00    	jbe    0x180049ab7
   1800499da:	8d 45 01             	lea    eax,[rbp+0x1]
   1800499dd:	48 89 b4 24 c0 02 00 	mov    QWORD PTR [rsp+0x2c0],rsi
   1800499e4:	00 
   1800499e5:	8b c8                	mov    ecx,eax
   1800499e7:	48 8d 15 f2 65 97 00 	lea    rdx,[rip+0x9765f2]        # 0x1809bffe0
   1800499ee:	41 b9 03 00 00 00    	mov    r9d,0x3
   1800499f4:	8b d8                	mov    ebx,eax
   1800499f6:	41 b8 ed 01 00 00    	mov    r8d,0x1ed
   1800499fc:	c7 44 24 20 00 00 00 	mov    DWORD PTR [rsp+0x20],0x0
   180049a03:	00 
   180049a04:	e8 f7 6e 94 00       	call   0x180990900
   180049a09:	41 b9 03 00 00 00    	mov    r9d,0x3
   180049a0f:	c7 44 24 20 00 00 00 	mov    DWORD PTR [rsp+0x20],0x0
   180049a16:	00 
   180049a17:	41 b8 ed 01 00 00    	mov    r8d,0x1ed
   180049a1d:	48 8d 15 bc 65 97 00 	lea    rdx,[rip+0x9765bc]        # 0x1809bffe0
   180049a24:	8b cb                	mov    ecx,ebx
   180049a26:	48 8b f0             	mov    rsi,rax
   180049a29:	e8 d2 6e 94 00       	call   0x180990900
   180049a2e:	44 8b c3             	mov    r8d,ebx
   180049a31:	33 d2                	xor    edx,edx
   180049a33:	48 8b ce             	mov    rcx,rsi
   180049a36:	48 8b f8             	mov    rdi,rax
   180049a39:	e8 22 73 94 00       	call   0x180990d60
   180049a3e:	44 8b c3             	mov    r8d,ebx
   180049a41:	33 d2                	xor    edx,edx
   180049a43:	48 8b cf             	mov    rcx,rdi
   180049a46:	e8 15 73 94 00       	call   0x180990d60
   180049a4b:	48 8b 0d 16 10 b4 00 	mov    rcx,QWORD PTR [rip+0xb41016]        # 0x180b8aa68
   180049a52:	4c 8d 05 d7 66 97 00 	lea    r8,[rip+0x9766d7]        # 0x1809c0130
   180049a59:	8b d5                	mov    edx,ebp
   180049a5b:	44 8b 89 c0 00 00 00 	mov    r9d,DWORD PTR [rcx+0xc0]
   180049a62:	48 8b ce             	mov    rcx,rsi
   180049a65:	e8 66 79 94 00       	call   0x1809913d0
   180049a6a:	44 8b c5             	mov    r8d,ebp
   180049a6d:	48 8d 54 24 70       	lea    rdx,[rsp+0x70]
   180049a72:	48 8b cf             	mov    rcx,rdi
   180049a75:	e8 46 7a 94 00       	call   0x1809914c0
   180049a7a:	44 8b c5             	mov    r8d,ebp
   180049a7d:	48 8b d6             	mov    rdx,rsi
   180049a80:	48 8b cf             	mov    rcx,rdi
   180049a83:	e8 38 7a 94 00       	call   0x1809914c0
   180049a88:	44 8b c5             	mov    r8d,ebp
   180049a8b:	48 8d 0d ae 48 97 00 	lea    rcx,[rip+0x9748ae]        # 0x1809be340
   180049a92:	48 8b d7             	mov    rdx,rdi
   180049a95:	e8 46 7e 94 00       	call   0x1809918e0
   180049a9a:	48 8b ce             	mov    rcx,rsi
   180049a9d:	e8 4e 6e 94 00       	call   0x1809908f0
   180049aa2:	48 8b cf             	mov    rcx,rdi
   180049aa5:	e8 46 6e 94 00       	call   0x1809908f0
   180049aaa:	48 8b b4 24 c0 02 00 	mov    rsi,QWORD PTR [rsp+0x2c0]
   180049ab1:	00 
   180049ab2:	e9 81 00 00 00       	jmp    0x180049b38
   180049ab7:	48 8b 05 aa 0f b4 00 	mov    rax,QWORD PTR [rip+0xb40faa]        # 0x180b8aa68
   180049abe:	4c 8d 05 6b 66 97 00 	lea    r8,[rip+0x97666b]        # 0x1809c0130
   180049ac5:	ba 00 01 00 00       	mov    edx,0x100
   180049aca:	48 8d 8c 24 90 01 00 	lea    rcx,[rsp+0x190]
   180049ad1:	00 
   180049ad2:	44 8b 88 c0 00 00 00 	mov    r9d,DWORD PTR [rax+0xc0]
   180049ad9:	e8 f2 78 94 00       	call   0x1809913d0
   180049ade:	44 8b c3             	mov    r8d,ebx
   180049ae1:	48 8d 94 24 90 01 00 	lea    rdx,[rsp+0x190]
   180049ae8:	00 
   180049ae9:	44 2b c7             	sub    r8d,edi
   180049aec:	48 8d 4c 24 70       	lea    rcx,[rsp+0x70]
   180049af1:	e8 ca 79 94 00       	call   0x1809914c0
   180049af6:	3b eb                	cmp    ebp,ebx
   180049af8:	0f 42 dd             	cmovb  ebx,ebp
   180049afb:	44 8b c3             	mov    r8d,ebx
   180049afe:	eb 27                	jmp    0x180049b27
   180049b00:	48 8b 05 61 0f b4 00 	mov    rax,QWORD PTR [rip+0xb40f61]        # 0x180b8aa68
   180049b07:	4c 8d 05 22 66 97 00 	lea    r8,[rip+0x976622]        # 0x1809c0130
   180049b0e:	ba 17 01 00 00       	mov    edx,0x117
   180049b13:	48 8d 4c 24 70       	lea    rcx,[rsp+0x70]
   180049b18:	44 8b 88 c0 00 00 00 	mov    r9d,DWORD PTR [rax+0xc0]
   180049b1f:	e8 ac 78 94 00       	call   0x1809913d0
   180049b24:	44 8b c0             	mov    r8d,eax
   180049b27:	48 8d 54 24 70       	lea    rdx,[rsp+0x70]
   180049b2c:	48 8d 0d 0d 48 97 00 	lea    rcx,[rip+0x97480d]        # 0x1809be340
   180049b33:	e8 a8 7d 94 00       	call   0x1809918e0
   180049b38:	48 8b 05 29 0f b4 00 	mov    rax,QWORD PTR [rip+0xb40f29]        # 0x180b8aa68
   180049b3f:	48 c7 44 24 50 b3 e3 	mov    QWORD PTR [rsp+0x50],0x66ade3b3
   180049b46:	ad 66 
   180049b48:	48 8b 4c 24 50       	mov    rcx,QWORD PTR [rsp+0x50]
   180049b4d:	8b 90 c0 00 00 00    	mov    edx,DWORD PTR [rax+0xc0]
   180049b53:	e8 98 29 5b 00       	call   0x1805fc4f0
   180049b58:	48 8b 05 09 0f b4 00 	mov    rax,QWORD PTR [rip+0xb40f09]        # 0x180b8aa68
   180049b5f:	48 8b 88 80 02 00 00 	mov    rcx,QWORD PTR [rax+0x280]
   180049b66:	8b 90 c0 00 00 00    	mov    edx,DWORD PTR [rax+0xc0]
   180049b6c:	48 81 c1 50 07 00 00 	add    rcx,0x750
   180049b73:	e8 68 fb ff ff       	call   0x1800496e0
   180049b78:	48 81 c4 90 02 00 00 	add    rsp,0x290
   180049b7f:	5f                   	pop    rdi
   180049b80:	5d                   	pop    rbp
   180049b81:	5b                   	pop    rbx
   180049b82:	e9 69 89 ff ff       	jmp    0x1800424f0

Game::Processor::Update

/home/phulin/c6-assets/DLC/Expansion2/Binaries/Win64/GameCore_XP2_FinalRelease.dll:     file format pei-x86-64


Disassembly of section .text:

000000018004afc0 <.text+0x49fc0>:
   18004afc0:	48 89 5c 24 18       	mov    QWORD PTR [rsp+0x18],rbx
   18004afc5:	55                   	push   rbp
   18004afc6:	56                   	push   rsi
   18004afc7:	57                   	push   rdi
   18004afc8:	41 54                	push   r12
   18004afca:	41 55                	push   r13
   18004afcc:	41 56                	push   r14
   18004afce:	41 57                	push   r15
   18004afd0:	48 8d ac 24 c0 fc ff 	lea    rbp,[rsp-0x340]
   18004afd7:	ff 
   18004afd8:	48 81 ec 40 04 00 00 	sub    rsp,0x440
   18004afdf:	c6 44 24 30 01       	mov    BYTE PTR [rsp+0x30],0x1
   18004afe4:	48 8d 15 5d 51 97 00 	lea    rdx,[rip+0x97515d]        # 0x1809c0148
   18004afeb:	45 33 ff             	xor    r15d,r15d
   18004afee:	c6 44 24 28 00       	mov    BYTE PTR [rsp+0x28],0x0
   18004aff3:	45 33 c9             	xor    r9d,r9d
   18004aff6:	44 89 7c 24 20       	mov    DWORD PTR [rsp+0x20],r15d
   18004affb:	41 b8 04 5f 9a 23    	mov    r8d,0x239a5f04
   18004b001:	48 8d 8d f0 01 00 00 	lea    rcx,[rbp+0x1f0]
   18004b008:	e8 e3 79 5b 00       	call   0x1806029f0
   18004b00d:	e8 6e 77 ff ff       	call   0x180042780
   18004b012:	45 32 f6             	xor    r14b,r14b
   18004b015:	44 88 3d 07 fb b3 00 	mov    BYTE PTR [rip+0xb3fb07],r15b        # 0x180b8ab23
   18004b01c:	40 32 f6             	xor    sil,sil
   18004b01f:	45 8d 67 01          	lea    r12d,[r15+0x1]
   18004b023:	40 38 35 f6 fa b3 00 	cmp    BYTE PTR [rip+0xb3faf6],sil        # 0x180b8ab20
   18004b02a:	74 09                	je     0x18004b035
   18004b02c:	44 38 3d ee fa b3 00 	cmp    BYTE PTR [rip+0xb3faee],r15b        # 0x180b8ab21
   18004b033:	75 11                	jne    0x18004b046
   18004b035:	44 38 3d e6 fa b3 00 	cmp    BYTE PTR [rip+0xb3fae6],r15b        # 0x180b8ab22
   18004b03c:	75 08                	jne    0x18004b046
   18004b03e:	45 8b ef             	mov    r13d,r15d
   18004b041:	e9 aa 00 00 00       	jmp    0x18004b0f0
   18004b046:	e8 45 e3 ff ff       	call   0x180049390
   18004b04b:	48 8d 4d 90          	lea    rcx,[rbp-0x70]
   18004b04f:	4c 89 bd ac 00 00 00 	mov    QWORD PTR [rbp+0xac],r15
   18004b056:	45 0f b6 f4          	movzx  r14d,r12b
   18004b05a:	44 89 bd b4 00 00 00 	mov    DWORD PTR [rbp+0xb4],r15d
   18004b061:	4c 89 bd a0 00 00 00 	mov    QWORD PTR [rbp+0xa0],r15
   18004b068:	44 89 bd a8 00 00 00 	mov    DWORD PTR [rbp+0xa8],r15d
   18004b06f:	e8 cc ec ff ff       	call   0x180049d40
   18004b074:	48 8b 95 a0 00 00 00 	mov    rdx,QWORD PTR [rbp+0xa0]
   18004b07b:	8b 85 b0 00 00 00    	mov    eax,DWORD PTR [rbp+0xb0]
   18004b081:	48 8b da             	mov    rbx,rdx
   18004b084:	48 8d 0c 82          	lea    rcx,[rdx+rax*4]
   18004b088:	48 3b d1             	cmp    rdx,rcx
   18004b08b:	74 42                	je     0x18004b0cf
   18004b08d:	0f 1f 00             	nop    DWORD PTR [rax]
   18004b090:	8b 0b                	mov    ecx,DWORD PTR [rbx]
   18004b092:	e8 69 9e ff ff       	call   0x180044f00
   18004b097:	48 8b c8             	mov    rcx,rax
   18004b09a:	48 8b f8             	mov    rdi,rax
   18004b09d:	e8 7e e7 2a 00       	call   0x1802f9820
   18004b0a2:	84 c0                	test   al,al
   18004b0a4:	74 0f                	je     0x18004b0b5
   18004b0a6:	44 38 bf 58 02 00 00 	cmp    BYTE PTR [rdi+0x258],r15b
   18004b0ad:	40 0f b6 f6          	movzx  esi,sil
   18004b0b1:	41 0f 45 f4          	cmovne esi,r12d
   18004b0b5:	8b 85 b0 00 00 00    	mov    eax,DWORD PTR [rbp+0xb0]
   18004b0bb:	48 83 c3 04          	add    rbx,0x4
   18004b0bf:	48 8b 95 a0 00 00 00 	mov    rdx,QWORD PTR [rbp+0xa0]
   18004b0c6:	48 8d 04 82          	lea    rax,[rdx+rax*4]
   18004b0ca:	48 3b d8             	cmp    rbx,rax
   18004b0cd:	75 c1                	jne    0x18004b090
   18004b0cf:	48 8d 45 9f          	lea    rax,[rbp-0x61]
   18004b0d3:	48 83 e0 f0          	and    rax,0xfffffffffffffff0
   18004b0d7:	48 3b d0             	cmp    rdx,rax
   18004b0da:	74 08                	je     0x18004b0e4
   18004b0dc:	48 8b ca             	mov    rcx,rdx
   18004b0df:	e8 0c 58 94 00       	call   0x1809908f0
   18004b0e4:	45 8b ef             	mov    r13d,r15d
   18004b0e7:	40 84 f6             	test   sil,sil
   18004b0ea:	0f 85 55 07 00 00    	jne    0x18004b845
   18004b0f0:	48 8d 4d 90          	lea    rcx,[rbp-0x70]
   18004b0f4:	48 c7 85 ac 00 00 00 	mov    QWORD PTR [rbp+0xac],0x0
   18004b0fb:	00 00 00 00 
   18004b0ff:	44 89 bd b4 00 00 00 	mov    DWORD PTR [rbp+0xb4],r15d
   18004b106:	4c 89 bd a0 00 00 00 	mov    QWORD PTR [rbp+0xa0],r15
   18004b10d:	44 89 bd a8 00 00 00 	mov    DWORD PTR [rbp+0xa8],r15d
   18004b114:	e8 27 ec ff ff       	call   0x180049d40
   18004b119:	48 8b bd a0 00 00 00 	mov    rdi,QWORD PTR [rbp+0xa0]
   18004b120:	8b 85 b0 00 00 00    	mov    eax,DWORD PTR [rbp+0xb0]
   18004b126:	48 8d 0c 87          	lea    rcx,[rdi+rax*4]
   18004b12a:	48 3b f9             	cmp    rdi,rcx
   18004b12d:	74 5d                	je     0x18004b18c
   18004b12f:	90                   	nop
   18004b130:	8b 0f                	mov    ecx,DWORD PTR [rdi]
   18004b132:	e8 c9 9d ff ff       	call   0x180044f00
   18004b137:	48 8b c8             	mov    rcx,rax
   18004b13a:	48 8b d8             	mov    rbx,rax
   18004b13d:	e8 de e6 2a 00       	call   0x1802f9820
   18004b142:	84 c0                	test   al,al
   18004b144:	40 0f b6 f6          	movzx  esi,sil
   18004b148:	41 0f 45 f4          	cmovne esi,r12d
   18004b14c:	80 bb 58 02 00 00 00 	cmp    BYTE PTR [rbx+0x258],0x0
   18004b153:	74 0f                	je     0x18004b164
   18004b155:	48 8b cb             	mov    rcx,rbx
   18004b158:	e8 c3 e6 2a 00       	call   0x1802f9820
   18004b15d:	84 c0                	test   al,al
   18004b15f:	41 8b c4             	mov    eax,r12d
   18004b162:	74 03                	je     0x18004b167
   18004b164:	41 8b c7             	mov    eax,r15d
   18004b167:	48 8b cb             	mov    rcx,rbx
   18004b16a:	44 0a f0             	or     r14b,al
   18004b16d:	e8 5e 31 01 00       	call   0x18005e2d0
   18004b172:	8b 8d b0 00 00 00    	mov    ecx,DWORD PTR [rbp+0xb0]
   18004b178:	48 83 c7 04          	add    rdi,0x4
   18004b17c:	48 8b 85 a0 00 00 00 	mov    rax,QWORD PTR [rbp+0xa0]
   18004b183:	48 8d 0c 88          	lea    rcx,[rax+rcx*4]
   18004b187:	48 3b f9             	cmp    rdi,rcx
   18004b18a:	75 a4                	jne    0x18004b130
   18004b18c:	48 8d 4d 90          	lea    rcx,[rbp-0x70]
   18004b190:	e8 0b ea ff ff       	call   0x180049ba0
   18004b195:	48 8b 9d a0 00 00 00 	mov    rbx,QWORD PTR [rbp+0xa0]
   18004b19c:	8b 85 b0 00 00 00    	mov    eax,DWORD PTR [rbp+0xb0]
   18004b1a2:	48 8d 0c 83          	lea    rcx,[rbx+rax*4]
   18004b1a6:	48 3b d9             	cmp    rbx,rcx
   18004b1a9:	74 4f                	je     0x18004b1fa
   18004b1ab:	0f 1f 44 00 00       	nop    DWORD PTR [rax+rax*1+0x0]
   18004b1b0:	8b 0b                	mov    ecx,DWORD PTR [rbx]
   18004b1b2:	e8 49 9d ff ff       	call   0x180044f00
   18004b1b7:	48 8b f8             	mov    rdi,rax
   18004b1ba:	80 b8 58 02 00 00 00 	cmp    BYTE PTR [rax+0x258],0x0
   18004b1c1:	74 0f                	je     0x18004b1d2
   18004b1c3:	48 8b c8             	mov    rcx,rax
   18004b1c6:	e8 55 e6 2a 00       	call   0x1802f9820
   18004b1cb:	84 c0                	test   al,al
   18004b1cd:	41 8b c4             	mov    eax,r12d
   18004b1d0:	74 03                	je     0x18004b1d5
   18004b1d2:	41 8b c7             	mov    eax,r15d
   18004b1d5:	48 8b cf             	mov    rcx,rdi
   18004b1d8:	44 0a f0             	or     r14b,al
   18004b1db:	e8 20 19 01 00       	call   0x18005cb00
   18004b1e0:	8b 8d b0 00 00 00    	mov    ecx,DWORD PTR [rbp+0xb0]
   18004b1e6:	48 83 c3 04          	add    rbx,0x4
   18004b1ea:	48 8b 85 a0 00 00 00 	mov    rax,QWORD PTR [rbp+0xa0]
   18004b1f1:	48 8d 0c 88          	lea    rcx,[rax+rcx*4]
   18004b1f5:	48 3b d9             	cmp    rbx,rcx
   18004b1f8:	75 b6                	jne    0x18004b1b0
   18004b1fa:	e8 61 1b 01 00       	call   0x18005cd60
   18004b1ff:	85 c0                	test   eax,eax
   18004b201:	0f 85 15 05 00 00    	jne    0x18004b71c
   18004b207:	e8 54 de ff ff       	call   0x180049060
   18004b20c:	44 0f b6 e0          	movzx  r12d,al
   18004b210:	e8 6b b9 2b 00       	call   0x180306b80
   18004b215:	48 8b 88 a8 0f 00 00 	mov    rcx,QWORD PTR [rax+0xfa8]
   18004b21c:	48 2b 88 a0 0f 00 00 	sub    rcx,QWORD PTR [rax+0xfa0]
   18004b223:	48 c1 f9 02          	sar    rcx,0x2
   18004b227:	85 c9                	test   ecx,ecx
   18004b229:	0f 85 df 01 00 00    	jne    0x18004b40e
   18004b22f:	48 8d 95 88 03 00 00 	lea    rdx,[rbp+0x388]
   18004b236:	48 c7 85 80 03 00 00 	mov    QWORD PTR [rbp+0x380],0x1
   18004b23d:	01 00 00 00 
   18004b241:	48 8d 8d 80 03 00 00 	lea    rcx,[rbp+0x380]
   18004b248:	48 c7 85 88 03 00 00 	mov    QWORD PTR [rbp+0x388],0x1
   18004b24f:	01 00 00 00 
   18004b253:	e8 28 66 94 00       	call   0x180991880
   18004b258:	48 83 bd 80 03 00 00 	cmp    QWORD PTR [rbp+0x380],0x0
   18004b25f:	00 
   18004b260:	0f 84 7a 01 00 00    	je     0x18004b3e0
   18004b266:	e8 a5 64 94 00       	call   0x180991710
   18004b26b:	48 8b d0             	mov    rdx,rax
   18004b26e:	48 8d 4c 24 50       	lea    rcx,[rsp+0x50]
   18004b273:	e8 88 65 94 00       	call   0x180991800
   18004b278:	48 8d 0d e1 4e 97 00 	lea    rcx,[rip+0x974ee1]        # 0x1809c0160
   18004b27f:	e8 0c 63 94 00       	call   0x180991590
   18004b284:	8b d8                	mov    ebx,eax
   18004b286:	83 f8 ff             	cmp    eax,0xffffffff
   18004b289:	0f 84 7f 01 00 00    	je     0x18004b40e
   18004b28f:	8b 4c 24 64          	mov    ecx,DWORD PTR [rsp+0x64]
   18004b293:	4c 8d 05 7e 30 97 00 	lea    r8,[rip+0x97307e]        # 0x1809be318
   18004b29a:	44 8b 4c 24 50       	mov    r9d,DWORD PTR [rsp+0x50]
   18004b29f:	ba 17 00 00 00       	mov    edx,0x17
   18004b2a4:	89 4c 24 40          	mov    DWORD PTR [rsp+0x40],ecx
   18004b2a8:	8b 4c 24 60          	mov    ecx,DWORD PTR [rsp+0x60]
   18004b2ac:	89 4c 24 38          	mov    DWORD PTR [rsp+0x38],ecx
   18004b2b0:	8b 4c 24 5c          	mov    ecx,DWORD PTR [rsp+0x5c]
   18004b2b4:	89 4c 24 30          	mov    DWORD PTR [rsp+0x30],ecx
   18004b2b8:	8b 4c 24 58          	mov    ecx,DWORD PTR [rsp+0x58]
   18004b2bc:	89 4c 24 28          	mov    DWORD PTR [rsp+0x28],ecx
   18004b2c0:	8b 4c 24 54          	mov    ecx,DWORD PTR [rsp+0x54]
   18004b2c4:	89 4c 24 20          	mov    DWORD PTR [rsp+0x20],ecx
   18004b2c8:	48 8d 8d c0 00 00 00 	lea    rcx,[rbp+0xc0]
   18004b2cf:	e8 fc 60 94 00       	call   0x1809913d0
   18004b2d4:	8b f8                	mov    edi,eax
   18004b2d6:	44 8d 3c 18          	lea    r15d,[rax+rbx*1]
   18004b2da:	41 81 ff 17 01 00 00 	cmp    r15d,0x117
   18004b2e1:	0f 86 b6 00 00 00    	jbe    0x18004b39d
   18004b2e7:	41 8d 47 01          	lea    eax,[r15+0x1]
   18004b2eb:	33 ff                	xor    edi,edi
   18004b2ed:	41 b8 80 02 00 00    	mov    r8d,0x280
   18004b2f3:	8b c8                	mov    ecx,eax
   18004b2f5:	48 8d 15 e4 4c 97 00 	lea    rdx,[rip+0x974ce4]        # 0x1809bffe0
   18004b2fc:	8b d8                	mov    ebx,eax
   18004b2fe:	89 7c 24 20          	mov    DWORD PTR [rsp+0x20],edi
   18004b302:	44 8d 4f 03          	lea    r9d,[rdi+0x3]
   18004b306:	e8 f5 55 94 00       	call   0x180990900
   18004b30b:	44 8d 4f 03          	lea    r9d,[rdi+0x3]
   18004b30f:	89 7c 24 20          	mov    DWORD PTR [rsp+0x20],edi
   18004b313:	41 b8 80 02 00 00    	mov    r8d,0x280
   18004b319:	48 8d 15 c0 4c 97 00 	lea    rdx,[rip+0x974cc0]        # 0x1809bffe0
   18004b320:	8b cb                	mov    ecx,ebx
   18004b322:	48 8b f0             	mov    rsi,rax
   18004b325:	e8 d6 55 94 00       	call   0x180990900
   18004b32a:	44 8b c3             	mov    r8d,ebx
   18004b32d:	33 d2                	xor    edx,edx
   18004b32f:	48 8b ce             	mov    rcx,rsi
   18004b332:	48 8b f8             	mov    rdi,rax
   18004b335:	e8 26 5a 94 00       	call   0x180990d60
   18004b33a:	44 8b c3             	mov    r8d,ebx
   18004b33d:	33 d2                	xor    edx,edx
   18004b33f:	48 8b cf             	mov    rcx,rdi
   18004b342:	e8 19 5a 94 00       	call   0x180990d60
   18004b347:	4c 8d 05 12 4e 97 00 	lea    r8,[rip+0x974e12]        # 0x1809c0160
   18004b34e:	41 8b d7             	mov    edx,r15d
   18004b351:	48 8b ce             	mov    rcx,rsi
   18004b354:	e8 77 60 94 00       	call   0x1809913d0
   18004b359:	45 8b c7             	mov    r8d,r15d
   18004b35c:	48 8d 95 c0 00 00 00 	lea    rdx,[rbp+0xc0]
   18004b363:	48 8b cf             	mov    rcx,rdi
   18004b366:	e8 55 61 94 00       	call   0x1809914c0
   18004b36b:	45 8b c7             	mov    r8d,r15d
   18004b36e:	48 8b d6             	mov    rdx,rsi
   18004b371:	48 8b cf             	mov    rcx,rdi
   18004b374:	e8 47 61 94 00       	call   0x1809914c0
   18004b379:	45 8b c7             	mov    r8d,r15d
   18004b37c:	48 8d 0d bd 2f 97 00 	lea    rcx,[rip+0x972fbd]        # 0x1809be340
   18004b383:	48 8b d7             	mov    rdx,rdi
   18004b386:	e8 55 65 94 00       	call   0x1809918e0
   18004b38b:	48 8b ce             	mov    rcx,rsi
   18004b38e:	e8 5d 55 94 00       	call   0x1809908f0
   18004b393:	48 8b cf             	mov    rcx,rdi
   18004b396:	e8 55 55 94 00       	call   0x1809908f0
   18004b39b:	eb 71                	jmp    0x18004b40e
   18004b39d:	4c 8d 05 bc 4d 97 00 	lea    r8,[rip+0x974dbc]        # 0x1809c0160
   18004b3a4:	ba 00 01 00 00       	mov    edx,0x100
   18004b3a9:	48 8d 8d 40 02 00 00 	lea    rcx,[rbp+0x240]
   18004b3b0:	e8 1b 60 94 00       	call   0x1809913d0
   18004b3b5:	41 b8 17 01 00 00    	mov    r8d,0x117
   18004b3bb:	48 8d 95 40 02 00 00 	lea    rdx,[rbp+0x240]
   18004b3c2:	44 2b c7             	sub    r8d,edi
   18004b3c5:	48 8d 8d c0 00 00 00 	lea    rcx,[rbp+0xc0]
   18004b3cc:	e8 ef 60 94 00       	call   0x1809914c0
   18004b3d1:	41 b8 17 01 00 00    	mov    r8d,0x117
   18004b3d7:	45 3b f8             	cmp    r15d,r8d
   18004b3da:	45 0f 42 c7          	cmovb  r8d,r15d
   18004b3de:	eb 1b                	jmp    0x18004b3fb
   18004b3e0:	4c 8d 05 79 4d 97 00 	lea    r8,[rip+0x974d79]        # 0x1809c0160
   18004b3e7:	ba 17 01 00 00       	mov    edx,0x117
   18004b3ec:	48 8d 8d c0 00 00 00 	lea    rcx,[rbp+0xc0]
   18004b3f3:	e8 d8 5f 94 00       	call   0x1809913d0
   18004b3f8:	44 8b c0             	mov    r8d,eax
   18004b3fb:	48 8d 95 c0 00 00 00 	lea    rdx,[rbp+0xc0]
   18004b402:	48 8d 0d 37 2f 97 00 	lea    rcx,[rip+0x972f37]        # 0x1809be340
   18004b409:	e8 d2 64 94 00       	call   0x1809918e0
   18004b40e:	e8 4d 19 01 00       	call   0x18005cd60
   18004b413:	85 c0                	test   eax,eax
   18004b415:	0f 85 f4 02 00 00    	jne    0x18004b70f
   18004b41b:	45 84 e4             	test   r12b,r12b
   18004b41e:	0f 85 ee 02 00 00    	jne    0x18004b712
   18004b424:	e8 57 b7 2b 00       	call   0x180306b80
   18004b429:	48 8b 88 a8 0f 00 00 	mov    rcx,QWORD PTR [rax+0xfa8]
   18004b430:	48 2b 88 a0 0f 00 00 	sub    rcx,QWORD PTR [rax+0xfa0]
   18004b437:	48 c1 f9 02          	sar    rcx,0x2
   18004b43b:	85 c9                	test   ecx,ecx
   18004b43d:	0f 84 cc 02 00 00    	je     0x18004b70f
   18004b443:	b1 01                	mov    cl,0x1
   18004b445:	e8 96 1a 01 00       	call   0x18005cee0
   18004b44a:	85 c0                	test   eax,eax
   18004b44c:	0f 84 b5 02 00 00    	je     0x18004b707
   18004b452:	48 8d 05 a3 52 b4 00 	lea    rax,[rip+0xb452a3]        # 0x180b906fc
   18004b459:	48 89 44 24 68       	mov    QWORD PTR [rsp+0x68],rax
   18004b45e:	48 8d 8d c0 00 00 00 	lea    rcx,[rbp+0xc0]
   18004b465:	48 89 44 24 70       	mov    QWORD PTR [rsp+0x70],rax
   18004b46a:	48 8d 05 8c 52 b4 00 	lea    rax,[rip+0xb4528c]        # 0x180b906fd
   18004b471:	48 89 44 24 78       	mov    QWORD PTR [rsp+0x78],rax
   18004b476:	e8 e5 1a 01 00       	call   0x18005cf60
   18004b47b:	48 8b d0             	mov    rdx,rax
   18004b47e:	48 8d 4c 24 68       	lea    rcx,[rsp+0x68]
   18004b483:	e8 c8 a0 17 00       	call   0x1801c5550
   18004b488:	48 8b 95 c0 00 00 00 	mov    rdx,QWORD PTR [rbp+0xc0]
   18004b48f:	48 85 d2             	test   rdx,rdx
   18004b492:	74 23                	je     0x18004b4b7
   18004b494:	48 3b 95 e0 00 00 00 	cmp    rdx,QWORD PTR [rbp+0xe0]
   18004b49b:	74 1a                	je     0x18004b4b7
   18004b49d:	4c 8b 85 d0 00 00 00 	mov    r8,QWORD PTR [rbp+0xd0]
   18004b4a4:	48 8d 8d d8 00 00 00 	lea    rcx,[rbp+0xd8]
   18004b4ab:	4c 2b c2             	sub    r8,rdx
   18004b4ae:	49 83 e0 fc          	and    r8,0xfffffffffffffffc
   18004b4b2:	e8 69 b9 fe ff       	call   0x180036e20
   18004b4b7:	48 8d 95 88 03 00 00 	lea    rdx,[rbp+0x388]
   18004b4be:	48 c7 85 80 03 00 00 	mov    QWORD PTR [rbp+0x380],0x1
   18004b4c5:	01 00 00 00 
   18004b4c9:	48 8d 8d 80 03 00 00 	lea    rcx,[rbp+0x380]
   18004b4d0:	48 c7 85 88 03 00 00 	mov    QWORD PTR [rbp+0x388],0x1
   18004b4d7:	01 00 00 00 
   18004b4db:	e8 a0 63 94 00       	call   0x180991880
   18004b4e0:	48 83 bd 80 03 00 00 	cmp    QWORD PTR [rbp+0x380],0x0
   18004b4e7:	00 
   18004b4e8:	0f 84 b0 01 00 00    	je     0x18004b69e
   18004b4ee:	e8 1d 62 94 00       	call   0x180991710
   18004b4f3:	48 8b d0             	mov    rdx,rax
   18004b4f6:	48 8d 4c 24 50       	lea    rcx,[rsp+0x50]
   18004b4fb:	e8 00 63 94 00       	call   0x180991800
   18004b500:	48 8b 5c 24 68       	mov    rbx,QWORD PTR [rsp+0x68]
   18004b505:	e8 56 18 01 00       	call   0x18005cd60
   18004b50a:	8b d0                	mov    edx,eax
   18004b50c:	48 8d 0d ad 4c 97 00 	lea    rcx,[rip+0x974cad]        # 0x1809c01c0
   18004b513:	4c 8b c3             	mov    r8,rbx
   18004b516:	e8 75 60 94 00       	call   0x180991590
   18004b51b:	8b d8                	mov    ebx,eax
   18004b51d:	83 f8 ff             	cmp    eax,0xffffffff
   18004b520:	0f 84 b8 01 00 00    	je     0x18004b6de
   18004b526:	8b 4c 24 64          	mov    ecx,DWORD PTR [rsp+0x64]
   18004b52a:	4c 8d 05 e7 2d 97 00 	lea    r8,[rip+0x972de7]        # 0x1809be318
   18004b531:	44 8b 4c 24 50       	mov    r9d,DWORD PTR [rsp+0x50]
   18004b536:	ba 17 00 00 00       	mov    edx,0x17
   18004b53b:	89 4c 24 40          	mov    DWORD PTR [rsp+0x40],ecx
   18004b53f:	8b 4c 24 60          	mov    ecx,DWORD PTR [rsp+0x60]
   18004b543:	89 4c 24 38          	mov    DWORD PTR [rsp+0x38],ecx
   18004b547:	8b 4c 24 5c          	mov    ecx,DWORD PTR [rsp+0x5c]
   18004b54b:	89 4c 24 30          	mov    DWORD PTR [rsp+0x30],ecx
   18004b54f:	8b 4c 24 58          	mov    ecx,DWORD PTR [rsp+0x58]
   18004b553:	89 4c 24 28          	mov    DWORD PTR [rsp+0x28],ecx
   18004b557:	8b 4c 24 54          	mov    ecx,DWORD PTR [rsp+0x54]
   18004b55b:	89 4c 24 20          	mov    DWORD PTR [rsp+0x20],ecx
   18004b55f:	48 8d 8d c0 00 00 00 	lea    rcx,[rbp+0xc0]
   18004b566:	e8 65 5e 94 00       	call   0x1809913d0
   18004b56b:	8b f8                	mov    edi,eax
   18004b56d:	44 8d 3c 18          	lea    r15d,[rax+rbx*1]
   18004b571:	41 81 ff 17 01 00 00 	cmp    r15d,0x117
   18004b578:	0f 86 cb 00 00 00    	jbe    0x18004b649
   18004b57e:	41 8d 47 01          	lea    eax,[r15+0x1]
   18004b582:	33 ff                	xor    edi,edi
   18004b584:	41 b8 8c 02 00 00    	mov    r8d,0x28c
   18004b58a:	8b c8                	mov    ecx,eax
   18004b58c:	48 8d 15 4d 4a 97 00 	lea    rdx,[rip+0x974a4d]        # 0x1809bffe0
   18004b593:	8b d8                	mov    ebx,eax
   18004b595:	89 7c 24 20          	mov    DWORD PTR [rsp+0x20],edi
   18004b599:	44 8d 4f 03          	lea    r9d,[rdi+0x3]
   18004b59d:	e8 5e 53 94 00       	call   0x180990900
   18004b5a2:	44 8d 4f 03          	lea    r9d,[rdi+0x3]
   18004b5a6:	89 7c 24 20          	mov    DWORD PTR [rsp+0x20],edi
   18004b5aa:	41 b8 8c 02 00 00    	mov    r8d,0x28c
   18004b5b0:	48 8d 15 29 4a 97 00 	lea    rdx,[rip+0x974a29]        # 0x1809bffe0
   18004b5b7:	8b cb                	mov    ecx,ebx
   18004b5b9:	4c 8b f0             	mov    r14,rax
   18004b5bc:	e8 3f 53 94 00       	call   0x180990900
   18004b5c1:	44 8b c3             	mov    r8d,ebx
   18004b5c4:	33 d2                	xor    edx,edx
   18004b5c6:	49 8b ce             	mov    rcx,r14
   18004b5c9:	48 8b f0             	mov    rsi,rax
   18004b5cc:	e8 8f 57 94 00       	call   0x180990d60
   18004b5d1:	44 8b c3             	mov    r8d,ebx
   18004b5d4:	33 d2                	xor    edx,edx
   18004b5d6:	48 8b ce             	mov    rcx,rsi
   18004b5d9:	e8 82 57 94 00       	call   0x180990d60
   18004b5de:	48 8b 5c 24 68       	mov    rbx,QWORD PTR [rsp+0x68]
   18004b5e3:	e8 78 17 01 00       	call   0x18005cd60
   18004b5e8:	44 8b c8             	mov    r9d,eax
   18004b5eb:	41 8b d7             	mov    edx,r15d
   18004b5ee:	4c 8d 05 cb 4b 97 00 	lea    r8,[rip+0x974bcb]        # 0x1809c01c0
   18004b5f5:	48 89 5c 24 20       	mov    QWORD PTR [rsp+0x20],rbx
   18004b5fa:	49 8b ce             	mov    rcx,r14
   18004b5fd:	e8 ce 5d 94 00       	call   0x1809913d0
   18004b602:	45 8b c7             	mov    r8d,r15d
   18004b605:	48 8d 95 c0 00 00 00 	lea    rdx,[rbp+0xc0]
   18004b60c:	48 8b ce             	mov    rcx,rsi
   18004b60f:	e8 ac 5e 94 00       	call   0x1809914c0
   18004b614:	45 8b c7             	mov    r8d,r15d
   18004b617:	49 8b d6             	mov    rdx,r14
   18004b61a:	48 8b ce             	mov    rcx,rsi
   18004b61d:	e8 9e 5e 94 00       	call   0x1809914c0
   18004b622:	45 8b c7             	mov    r8d,r15d
   18004b625:	48 8d 0d 14 2d 97 00 	lea    rcx,[rip+0x972d14]        # 0x1809be340
   18004b62c:	48 8b d6             	mov    rdx,rsi
   18004b62f:	e8 ac 62 94 00       	call   0x1809918e0
   18004b634:	49 8b ce             	mov    rcx,r14
   18004b637:	e8 b4 52 94 00       	call   0x1809908f0
   18004b63c:	48 8b ce             	mov    rcx,rsi
   18004b63f:	e8 ac 52 94 00       	call   0x1809908f0
   18004b644:	e9 95 00 00 00       	jmp    0x18004b6de
   18004b649:	48 8b 5c 24 68       	mov    rbx,QWORD PTR [rsp+0x68]
   18004b64e:	e8 0d 17 01 00       	call   0x18005cd60
   18004b653:	44 8b c8             	mov    r9d,eax
   18004b656:	48 89 5c 24 20       	mov    QWORD PTR [rsp+0x20],rbx
   18004b65b:	4c 8d 05 5e 4b 97 00 	lea    r8,[rip+0x974b5e]        # 0x1809c01c0
   18004b662:	ba 00 01 00 00       	mov    edx,0x100
   18004b667:	48 8d 8d 40 02 00 00 	lea    rcx,[rbp+0x240]
   18004b66e:	e8 5d 5d 94 00       	call   0x1809913d0
   18004b673:	41 b8 17 01 00 00    	mov    r8d,0x117
   18004b679:	48 8d 95 40 02 00 00 	lea    rdx,[rbp+0x240]
   18004b680:	44 2b c7             	sub    r8d,edi
   18004b683:	48 8d 8d c0 00 00 00 	lea    rcx,[rbp+0xc0]
   18004b68a:	e8 31 5e 94 00       	call   0x1809914c0
   18004b68f:	41 b8 17 01 00 00    	mov    r8d,0x117
   18004b695:	45 3b f8             	cmp    r15d,r8d
   18004b698:	45 0f 42 c7          	cmovb  r8d,r15d
   18004b69c:	eb 2d                	jmp    0x18004b6cb
   18004b69e:	48 8b 5c 24 68       	mov    rbx,QWORD PTR [rsp+0x68]
   18004b6a3:	e8 b8 16 01 00       	call   0x18005cd60
   18004b6a8:	44 8b c8             	mov    r9d,eax
   18004b6ab:	48 89 5c 24 20       	mov    QWORD PTR [rsp+0x20],rbx
   18004b6b0:	4c 8d 05 09 4b 97 00 	lea    r8,[rip+0x974b09]        # 0x1809c01c0
   18004b6b7:	ba 17 01 00 00       	mov    edx,0x117
   18004b6bc:	48 8d 8d c0 00 00 00 	lea    rcx,[rbp+0xc0]
   18004b6c3:	e8 08 5d 94 00       	call   0x1809913d0
   18004b6c8:	44 8b c0             	mov    r8d,eax
   18004b6cb:	48 8d 95 c0 00 00 00 	lea    rdx,[rbp+0xc0]
   18004b6d2:	48 8d 0d 67 2c 97 00 	lea    rcx,[rip+0x972c67]        # 0x1809be340
   18004b6d9:	e8 02 62 94 00       	call   0x1809918e0
   18004b6de:	48 8b 4c 24 78       	mov    rcx,QWORD PTR [rsp+0x78]
   18004b6e3:	48 8b 54 24 68       	mov    rdx,QWORD PTR [rsp+0x68]
   18004b6e8:	48 8b c1             	mov    rax,rcx
   18004b6eb:	48 2b c2             	sub    rax,rdx
   18004b6ee:	48 83 f8 01          	cmp    rax,0x1
   18004b6f2:	7e 13                	jle    0x18004b707
   18004b6f4:	48 85 d2             	test   rdx,rdx
   18004b6f7:	74 0e                	je     0x18004b707
   18004b6f9:	2b ca                	sub    ecx,edx
   18004b6fb:	44 8b c1             	mov    r8d,ecx
   18004b6fe:	48 8d 4d 80          	lea    rcx,[rbp-0x80]
   18004b702:	e8 19 b7 fe ff       	call   0x180036e20
   18004b707:	e8 e4 e1 ff ff       	call   0x1800498f0
   18004b70c:	41 b6 01             	mov    r14b,0x1
   18004b70f:	45 84 e4             	test   r12b,r12b
   18004b712:	40 0f 94 c6          	sete   sil
   18004b716:	41 bc 01 00 00 00    	mov    r12d,0x1
   18004b71c:	48 8b 0d 45 f3 b3 00 	mov    rcx,QWORD PTR [rip+0xb3f345]        # 0x180b8aa68
   18004b723:	48 8b 89 50 02 00 00 	mov    rcx,QWORD PTR [rcx+0x250]
   18004b72a:	e8 b1 5f 45 00       	call   0x1804a16e0
   18004b72f:	40 84 f6             	test   sil,sil
   18004b732:	0f 85 9d 00 00 00    	jne    0x18004b7d5
   18004b738:	48 8b 0d 39 f3 b3 00 	mov    rcx,QWORD PTR [rip+0xb3f339]        # 0x180b8aa78
   18004b73f:	e8 ac a0 11 00       	call   0x1801657f0
   18004b744:	84 c0                	test   al,al
   18004b746:	0f 85 80 00 00 00    	jne    0x18004b7cc
   18004b74c:	e8 2f b4 2b 00       	call   0x180306b80
   18004b751:	48 8b 78 50          	mov    rdi,QWORD PTR [rax+0x50]
   18004b755:	4c 8b 78 58          	mov    r15,QWORD PTR [rax+0x58]
   18004b759:	49 3b ff             	cmp    rdi,r15
   18004b75c:	74 6e                	je     0x18004b7cc
   18004b75e:	66 90                	xchg   ax,ax
   18004b760:	48 8b 07             	mov    rax,QWORD PTR [rdi]
   18004b763:	80 b8 88 02 00 00 00 	cmp    BYTE PTR [rax+0x288],0x0
   18004b76a:	74 57                	je     0x18004b7c3
   18004b76c:	8b 98 d8 00 00 00    	mov    ebx,DWORD PTR [rax+0xd8]
   18004b772:	ba 02 00 00 00       	mov    edx,0x2
   18004b777:	8b cb                	mov    ecx,ebx
   18004b779:	e8 02 a9 13 00       	call   0x180186080
   18004b77e:	84 c0                	test   al,al
   18004b780:	0f 85 8b 00 00 00    	jne    0x18004b811
   18004b786:	ba 02 00 00 00       	mov    edx,0x2
   18004b78b:	8b cb                	mov    ecx,ebx
   18004b78d:	e8 ce a6 13 00       	call   0x180185e60
   18004b792:	84 c0                	test   al,al
   18004b794:	75 7b                	jne    0x18004b811
   18004b796:	83 ca ff             	or     edx,0xffffffff
   18004b799:	8b cb                	mov    ecx,ebx
   18004b79b:	e8 30 a6 13 00       	call   0x180185dd0
   18004b7a0:	84 c0                	test   al,al
   18004b7a2:	75 6d                	jne    0x18004b811
   18004b7a4:	ba 02 00 00 00       	mov    edx,0x2
   18004b7a9:	8b cb                	mov    ecx,ebx
   18004b7ab:	e8 f0 a6 13 00       	call   0x180185ea0
   18004b7b0:	84 c0                	test   al,al
   18004b7b2:	75 5d                	jne    0x18004b811
   18004b7b4:	41 8b d4             	mov    edx,r12d
   18004b7b7:	41 8b cc             	mov    ecx,r12d
   18004b7ba:	e8 f1 a6 13 00       	call   0x180185eb0
   18004b7bf:	84 c0                	test   al,al
   18004b7c1:	75 4e                	jne    0x18004b811
   18004b7c3:	48 83 c7 08          	add    rdi,0x8
   18004b7c7:	49 3b ff             	cmp    rdi,r15
   18004b7ca:	75 94                	jne    0x18004b760
   18004b7cc:	41 ff c5             	inc    r13d
   18004b7cf:	41 83 fd 64          	cmp    r13d,0x64
   18004b7d3:	7d 41                	jge    0x18004b816
   18004b7d5:	48 8d 85 cf 00 00 00 	lea    rax,[rbp+0xcf]
   18004b7dc:	48 a9 f0 ff ff ff    	test   rax,0xfffffffffffffff0
   18004b7e2:	74 07                	je     0x18004b7eb
   18004b7e4:	33 c9                	xor    ecx,ecx
   18004b7e6:	e8 05 51 94 00       	call   0x1809908f0
   18004b7eb:	48 8b 8d a0 00 00 00 	mov    rcx,QWORD PTR [rbp+0xa0]
   18004b7f2:	48 8d 45 9f          	lea    rax,[rbp-0x61]
   18004b7f6:	48 83 e0 f0          	and    rax,0xfffffffffffffff0
   18004b7fa:	48 3b c8             	cmp    rcx,rax
   18004b7fd:	74 05                	je     0x18004b804
   18004b7ff:	e8 ec 50 94 00       	call   0x1809908f0
   18004b804:	40 84 f6             	test   sil,sil
   18004b807:	75 3c                	jne    0x18004b845
   18004b809:	45 33 ff             	xor    r15d,r15d
   18004b80c:	e9 df f8 ff ff       	jmp    0x18004b0f0
   18004b811:	40 b6 01             	mov    sil,0x1
   18004b814:	eb bf                	jmp    0x18004b7d5
   18004b816:	48 8d 85 cf 00 00 00 	lea    rax,[rbp+0xcf]
   18004b81d:	48 a9 f0 ff ff ff    	test   rax,0xfffffffffffffff0
   18004b823:	74 07                	je     0x18004b82c
   18004b825:	33 c9                	xor    ecx,ecx
   18004b827:	e8 c4 50 94 00       	call   0x1809908f0
   18004b82c:	48 8b 8d a0 00 00 00 	mov    rcx,QWORD PTR [rbp+0xa0]
   18004b833:	48 8d 45 9f          	lea    rax,[rbp-0x61]
   18004b837:	48 83 e0 f0          	and    rax,0xfffffffffffffff0
   18004b83b:	48 3b c8             	cmp    rcx,rax
   18004b83e:	74 05                	je     0x18004b845
   18004b840:	e8 ab 50 94 00       	call   0x1809908f0
   18004b845:	45 84 f6             	test   r14b,r14b
   18004b848:	0f 94 85 31 02 00 00 	sete   BYTE PTR [rbp+0x231]
   18004b84f:	e8 2c 6f ff ff       	call   0x180042780
   18004b854:	48 8d 8d f0 01 00 00 	lea    rcx,[rbp+0x1f0]
   18004b85b:	e8 30 72 5b 00       	call   0x180602a90
   18004b860:	48 8b 9c 24 90 04 00 	mov    rbx,QWORD PTR [rsp+0x490]
   18004b867:	00 
   18004b868:	48 81 c4 40 04 00 00 	add    rsp,0x440
   18004b86f:	41 5f                	pop    r15
   18004b871:	41 5e                	pop    r14
   18004b873:	41 5d                	pop    r13
   18004b875:	41 5c                	pop    r12
   18004b877:	5f                   	pop    rdi
   18004b878:	5e                   	pop    rsi
   18004b879:	5d                   	pop    rbp
   18004b87a:	c3                   	ret

Player::Processor::DoTurn

/home/phulin/c6-assets/DLC/Expansion2/Binaries/Win64/GameCore_XP2_FinalRelease.dll:     file format pei-x86-64


Disassembly of section .text:

000000018005ca10 <.text+0x5ba10>:
   18005ca10:	48 89 5c 24 08       	mov    QWORD PTR [rsp+0x8],rbx
   18005ca15:	57                   	push   rdi
   18005ca16:	48 83 ec 20          	sub    rsp,0x20
   18005ca1a:	48 8b d9             	mov    rbx,rcx
   18005ca1d:	48 8b 89 88 07 00 00 	mov    rcx,QWORD PTR [rcx+0x788]
   18005ca24:	e8 d7 7f 2e 00       	call   0x180344a00
   18005ca29:	48 63 83 d8 00 00 00 	movsxd rax,DWORD PTR [rbx+0xd8]
   18005ca30:	48 8d 3d c9 35 fa ff 	lea    rdi,[rip+0xfffffffffffa35c9]        # 0x180000000
   18005ca37:	48 8b cb             	mov    rcx,rbx
   18005ca3a:	ff 84 87 30 af b8 00 	inc    DWORD PTR [rdi+rax*4+0xb8af30]
   18005ca41:	48 63 83 d8 00 00 00 	movsxd rax,DWORD PTR [rbx+0xd8]
   18005ca48:	ff 84 87 30 b0 b8 00 	inc    DWORD PTR [rdi+rax*4+0xb8b030]
   18005ca4f:	e8 cc cd 29 00       	call   0x1802f9820
   18005ca54:	84 c0                	test   al,al
   18005ca56:	75 0e                	jne    0x18005ca66
   18005ca58:	48 63 83 d8 00 00 00 	movsxd rax,DWORD PTR [rbx+0xd8]
   18005ca5f:	ff 84 87 30 b1 b8 00 	inc    DWORD PTR [rdi+rax*4+0xb8b130]
   18005ca66:	48 8b 5c 24 30       	mov    rbx,QWORD PTR [rsp+0x30]
   18005ca6b:	48 83 c4 20          	add    rsp,0x20
   18005ca6f:	5f                   	pop    rdi
   18005ca70:	c3                   	ret

Exports::Context::RequestAction

/home/phulin/c6-assets/DLC/Expansion2/Binaries/Win64/GameCore_XP2_FinalRelease.dll:     file format pei-x86-64


Disassembly of section .text:

0000000180590830 <.text+0x58f830>:
   180590830:	48 89 5c 24 08       	mov    QWORD PTR [rsp+0x8],rbx
   180590835:	48 89 74 24 10       	mov    QWORD PTR [rsp+0x10],rsi
   18059083a:	57                   	push   rdi
   18059083b:	48 83 ec 70          	sub    rsp,0x70
   18059083f:	48 8b f1             	mov    rsi,rcx
   180590842:	41 8b d8             	mov    ebx,r8d
   180590845:	48 8d 4c 24 20       	lea    rcx,[rsp+0x20]
   18059084a:	8b fa                	mov    edi,edx
   18059084c:	e8 0f 47 41 00       	call   0x1809a4f60
   180590851:	4c 8d 4c 24 20       	lea    r9,[rsp+0x20]
   180590856:	44 8b c3             	mov    r8d,ebx
   180590859:	8b d7                	mov    edx,edi
   18059085b:	48 8b ce             	mov    rcx,rsi
   18059085e:	e8 1d 00 00 00       	call   0x180590880
   180590863:	48 8d 4c 24 20       	lea    rcx,[rsp+0x20]
   180590868:	e8 73 47 41 00       	call   0x1809a4fe0
   18059086d:	4c 8d 5c 24 70       	lea    r11,[rsp+0x70]
   180590872:	49 8b 5b 10          	mov    rbx,QWORD PTR [r11+0x10]
   180590876:	49 8b 73 18          	mov    rsi,QWORD PTR [r11+0x18]
   18059087a:	49 8b e3             	mov    rsp,r11
   18059087d:	5f                   	pop    rdi
   18059087e:	c3                   	ret

Exports::Context::RequestAction

/home/phulin/c6-assets/DLC/Expansion2/Binaries/Win64/GameCore_XP2_FinalRelease.dll:     file format pei-x86-64


Disassembly of section .text:

0000000180590880 <.text+0x58f880>:
   180590880:	48 89 4c 24 08       	mov    QWORD PTR [rsp+0x8],rcx
   180590885:	55                   	push   rbp
   180590886:	53                   	push   rbx
   180590887:	57                   	push   rdi
   180590888:	41 54                	push   r12
   18059088a:	41 55                	push   r13
   18059088c:	48 8d ac 24 c0 fe ff 	lea    rbp,[rsp-0x140]
   180590893:	ff 
   180590894:	48 81 ec 40 02 00 00 	sub    rsp,0x240
   18059089b:	4c 8b e9             	mov    r13,rcx
   18059089e:	49 8b d9             	mov    rbx,r9
   1805908a1:	48 8b 49 08          	mov    rcx,QWORD PTR [rcx+0x8]
   1805908a5:	45 8b e0             	mov    r12d,r8d
   1805908a8:	8b fa                	mov    edi,edx
   1805908aa:	e8 a1 54 ab ff       	call   0x180045d50
   1805908af:	48 8b c8             	mov    rcx,rax
   1805908b2:	e8 f9 ac 07 00       	call   0x18060b5b0
   1805908b7:	84 c0                	test   al,al
   1805908b9:	0f 85 1a 0c 00 00    	jne    0x1805914d9
   1805908bf:	4c 89 bc 24 88 02 00 	mov    QWORD PTR [rsp+0x288],r15
   1805908c6:	00 
   1805908c7:	45 8b fc             	mov    r15d,r12d
   1805908ca:	41 83 fc ff          	cmp    r12d,0xffffffff
   1805908ce:	75 08                	jne    0x1805908d8
   1805908d0:	e8 8b 45 ab ff       	call   0x180044e60
   1805908d5:	44 8b f8             	mov    r15d,eax
   1805908d8:	48 89 b4 24 78 02 00 	mov    QWORD PTR [rsp+0x278],rsi
   1805908df:	00 
   1805908e0:	4c 89 b4 24 80 02 00 	mov    QWORD PTR [rsp+0x280],r14
   1805908e7:	00 
   1805908e8:	81 ff 78 3c 1b 7d    	cmp    edi,0x7d1b3c78
   1805908ee:	0f 87 67 05 00 00    	ja     0x180590e5b
   1805908f4:	0f 84 54 04 00 00    	je     0x180590d4e
   1805908fa:	81 ff a4 a9 e7 41    	cmp    edi,0x41e7a9a4
   180590900:	0f 87 55 03 00 00    	ja     0x180590c5b
   180590906:	0f 84 22 03 00 00    	je     0x180590c2e
   18059090c:	81 ff bf a3 c6 1e    	cmp    edi,0x1ec6a3bf
   180590912:	74 6d                	je     0x180590981
   180590914:	81 ff a3 bd ca 23    	cmp    edi,0x23cabda3
   18059091a:	74 48                	je     0x180590964
   18059091c:	81 ff b5 a6 c9 2c    	cmp    edi,0x2cc9a6b5
   180590922:	0f 85 99 0b 00 00    	jne    0x1805914c1
   180590928:	48 8b 03             	mov    rax,QWORD PTR [rbx]
   18059092b:	ba 75 b0 bf 34       	mov    edx,0x34bfb075
   180590930:	48 8b cb             	mov    rcx,rbx
   180590933:	bf 84 12 fa b6       	mov    edi,0xb6fa1284
   180590938:	ff 50 60             	call   QWORD PTR [rax+0x60]
   18059093b:	48 85 c0             	test   rax,rax
   18059093e:	74 0a                	je     0x18059094a
   180590940:	48 8b c8             	mov    rcx,rax
   180590943:	e8 a8 58 07 00       	call   0x1806061f0
   180590948:	8b f8                	mov    edi,eax
   18059094a:	44 8b c7             	mov    r8d,edi
   18059094d:	48 8d 4c 24 30       	lea    rcx,[rsp+0x30]
   180590952:	41 8b d7             	mov    edx,r15d
   180590955:	e8 c6 8e aa ff       	call   0x180039820
   18059095a:	48 8d 4c 24 30       	lea    rcx,[rsp+0x30]
   18059095f:	e9 58 0b 00 00       	jmp    0x1805914bc
   180590964:	45 33 c9             	xor    r9d,r9d
   180590967:	41 b0 01             	mov    r8b,0x1
   18059096a:	41 8b d7             	mov    edx,r15d
   18059096d:	48 8d 4c 24 30       	lea    rcx,[rsp+0x30]
   180590972:	e8 59 79 aa ff       	call   0x1800382d0
   180590977:	48 8d 4c 24 30       	lea    rcx,[rsp+0x30]
   18059097c:	e9 3b 0b 00 00       	jmp    0x1805914bc
   180590981:	48 8d 05 74 fd 5f 00 	lea    rax,[rip+0x5ffd74]        # 0x180b906fc
   180590988:	ba 3e dc ca cf       	mov    edx,0xcfcadc3e
   18059098d:	48 89 44 24 30       	mov    QWORD PTR [rsp+0x30],rax
   180590992:	48 8b cb             	mov    rcx,rbx
   180590995:	48 89 44 24 38       	mov    QWORD PTR [rsp+0x38],rax
   18059099a:	48 8d 05 5c fd 5f 00 	lea    rax,[rip+0x5ffd5c]        # 0x180b906fd
   1805909a1:	48 89 44 24 40       	mov    QWORD PTR [rsp+0x40],rax
   1805909a6:	48 8b 03             	mov    rax,QWORD PTR [rbx]
   1805909a9:	ff 50 60             	call   QWORD PTR [rax+0x60]
   1805909ac:	48 83 ce ff          	or     rsi,0xffffffffffffffff
   1805909b0:	48 85 c0             	test   rax,rax
   1805909b3:	0f 84 ab 00 00 00    	je     0x180590a64
   1805909b9:	8b 48 14             	mov    ecx,DWORD PTR [rax+0x14]
   1805909bc:	c1 e9 1d             	shr    ecx,0x1d
   1805909bf:	f6 c1 01             	test   cl,0x1
   1805909c2:	0f 84 9c 00 00 00    	je     0x180590a64
   1805909c8:	66 83 38 05          	cmp    WORD PTR [rax],0x5
   1805909cc:	0f 85 92 00 00 00    	jne    0x180590a64
   1805909d2:	4c 8b 70 08          	mov    r14,QWORD PTR [rax+0x8]
   1805909d6:	4d 85 f6             	test   r14,r14
   1805909d9:	0f 84 85 00 00 00    	je     0x180590a64
   1805909df:	48 8b fe             	mov    rdi,rsi
   1805909e2:	48 ff c7             	inc    rdi
   1805909e5:	41 80 3c 3e 00       	cmp    BYTE PTR [r14+rdi*1],0x0
   1805909ea:	75 f6                	jne    0x1805909e2
   1805909ec:	4c 8b 44 24 38       	mov    r8,QWORD PTR [rsp+0x38]
   1805909f1:	49 03 fe             	add    rdi,r14
   1805909f4:	48 8b 4c 24 30       	mov    rcx,QWORD PTR [rsp+0x30]
   1805909f9:	4c 8b ff             	mov    r15,rdi
   1805909fc:	41 8b c0             	mov    eax,r8d
   1805909ff:	4d 2b fe             	sub    r15,r14
   180590a02:	2b c1                	sub    eax,ecx
   180590a04:	49 8b d6             	mov    rdx,r14
   180590a07:	44 3b f8             	cmp    r15d,eax
   180590a0a:	77 37                	ja     0x180590a43
   180590a0c:	4d 8b c7             	mov    r8,r15
   180590a0f:	ff 15 eb 6b 42 00    	call   QWORD PTR [rip+0x426beb]        # 0x1809b7600
   180590a15:	4c 8b 74 24 30       	mov    r14,QWORD PTR [rsp+0x30]
   180590a1a:	4c 8b 64 24 38       	mov    r12,QWORD PTR [rsp+0x38]
   180590a1f:	4d 03 f7             	add    r14,r15
   180590a22:	4d 3b f4             	cmp    r14,r12
   180590a25:	74 3d                	je     0x180590a64
   180590a27:	41 b8 01 00 00 00    	mov    r8d,0x1
   180590a2d:	49 8b d4             	mov    rdx,r12
   180590a30:	49 8b ce             	mov    rcx,r14
   180590a33:	ff 15 c7 6b 42 00    	call   QWORD PTR [rip+0x426bc7]        # 0x1809b7600
   180590a39:	4d 2b f4             	sub    r14,r12
   180590a3c:	4c 01 74 24 38       	add    QWORD PTR [rsp+0x38],r14
   180590a41:	eb 21                	jmp    0x180590a64
   180590a43:	4c 2b c1             	sub    r8,rcx
   180590a46:	ff 15 b4 6b 42 00    	call   QWORD PTR [rip+0x426bb4]        # 0x1809b7600
   180590a4c:	8b 54 24 38          	mov    edx,DWORD PTR [rsp+0x38]
   180590a50:	48 8d 4c 24 30       	lea    rcx,[rsp+0x30]
   180590a55:	2b 54 24 30          	sub    edx,DWORD PTR [rsp+0x30]
   180590a59:	4c 8b c7             	mov    r8,rdi
   180590a5c:	49 03 d6             	add    rdx,r14
   180590a5f:	e8 1c 40 a8 ff       	call   0x180014a80
   180590a64:	48 8b 03             	mov    rax,QWORD PTR [rbx]
   180590a67:	33 ff                	xor    edi,edi
   180590a69:	ba cf b5 65 60       	mov    edx,0x6065b5cf
   180590a6e:	48 8b cb             	mov    rcx,rbx
   180590a71:	44 8b ff             	mov    r15d,edi
   180590a74:	ff 50 60             	call   QWORD PTR [rax+0x60]
   180590a77:	48 8b d0             	mov    rdx,rax
   180590a7a:	41 be 2e 00 30 00    	mov    r14d,0x30002e
   180590a80:	48 85 c0             	test   rax,rax
   180590a83:	74 1f                	je     0x180590aa4
   180590a85:	8b 48 14             	mov    ecx,DWORD PTR [rax+0x14]
   180590a88:	c1 e9 1d             	shr    ecx,0x1d
   180590a8b:	f6 c1 01             	test   cl,0x1
   180590a8e:	75 14                	jne    0x180590aa4
   180590a90:	0f b7 00             	movzx  eax,WORD PTR [rax]
   180590a93:	66 83 f8 15          	cmp    ax,0x15
   180590a97:	77 0b                	ja     0x180590aa4
   180590a99:	98                   	cwde
   180590a9a:	41 0f a3 c6          	bt     r14d,eax
   180590a9e:	73 04                	jae    0x180590aa4
   180590aa0:	44 8b 7a 08          	mov    r15d,DWORD PTR [rdx+0x8]
   180590aa4:	48 8b 03             	mov    rax,QWORD PTR [rbx]
   180590aa7:	ba cb a5 ed f4       	mov    edx,0xf4eda5cb
   180590aac:	48 8b cb             	mov    rcx,rbx
   180590aaf:	ff 50 60             	call   QWORD PTR [rax+0x60]
   180590ab2:	48 85 c0             	test   rax,rax
   180590ab5:	74 24                	je     0x180590adb
   180590ab7:	8b 48 14             	mov    ecx,DWORD PTR [rax+0x14]
   180590aba:	c1 e9 1d             	shr    ecx,0x1d
   180590abd:	f6 c1 01             	test   cl,0x1
   180590ac0:	75 17                	jne    0x180590ad9
   180590ac2:	0f b7 08             	movzx  ecx,WORD PTR [rax]
   180590ac5:	66 83 f9 15          	cmp    cx,0x15
   180590ac9:	77 0e                	ja     0x180590ad9
   180590acb:	0f bf c9             	movsx  ecx,cx
   180590ace:	41 0f a3 ce          	bt     r14d,ecx
   180590ad2:	73 05                	jae    0x180590ad9
   180590ad4:	8b 70 08             	mov    esi,DWORD PTR [rax+0x8]
   180590ad7:	eb 02                	jmp    0x180590adb
   180590ad9:	8b f7                	mov    esi,edi
   180590adb:	48 8d 4d 40          	lea    rcx,[rbp+0x40]
   180590adf:	ff 15 bb 68 42 00    	call   QWORD PTR [rip+0x4268bb]        # 0x1809b73a0
   180590ae5:	48 8d 4d c0          	lea    rcx,[rbp-0x40]
   180590ae9:	ff 15 b1 68 42 00    	call   QWORD PTR [rip+0x4268b1]        # 0x1809b73a0
   180590aef:	41 83 ff 01          	cmp    r15d,0x1
   180590af3:	75 37                	jne    0x180590b2c
   180590af5:	48 8d 15 b4 fa 47 00 	lea    rdx,[rip+0x47fab4]        # 0x180a105b0
   180590afc:	48 8d 8d c0 00 00 00 	lea    rcx,[rbp+0xc0]
   180590b03:	ff 15 af 68 42 00    	call   QWORD PTR [rip+0x4268af]        # 0x1809b73b8
   180590b09:	48 8b d0             	mov    rdx,rax
   180590b0c:	48 8d 4d 40          	lea    rcx,[rbp+0x40]
   180590b10:	ff 15 02 68 42 00    	call   QWORD PTR [rip+0x426802]        # 0x1809b7318
   180590b16:	48 8d 8d c0 00 00 00 	lea    rcx,[rbp+0xc0]
   180590b1d:	ff 15 fd 67 42 00    	call   QWORD PTR [rip+0x4267fd]        # 0x1809b7320
   180590b23:	48 8d 15 ae fa 47 00 	lea    rdx,[rip+0x47faae]        # 0x180a105d8
   180590b2a:	eb 3b                	jmp    0x180590b67
   180590b2c:	41 83 ff 02          	cmp    r15d,0x2
   180590b30:	75 5c                	jne    0x180590b8e
   180590b32:	48 8d 15 cf fa 47 00 	lea    rdx,[rip+0x47facf]        # 0x180a10608
   180590b39:	48 8d 8d c0 00 00 00 	lea    rcx,[rbp+0xc0]
   180590b40:	ff 15 72 68 42 00    	call   QWORD PTR [rip+0x426872]        # 0x1809b73b8
   180590b46:	48 8b d0             	mov    rdx,rax
   180590b49:	48 8d 4d 40          	lea    rcx,[rbp+0x40]
   180590b4d:	ff 15 c5 67 42 00    	call   QWORD PTR [rip+0x4267c5]        # 0x1809b7318
   180590b53:	48 8d 8d c0 00 00 00 	lea    rcx,[rbp+0xc0]
   180590b5a:	ff 15 c0 67 42 00    	call   QWORD PTR [rip+0x4267c0]        # 0x1809b7320
   180590b60:	48 8d 15 c9 fa 47 00 	lea    rdx,[rip+0x47fac9]        # 0x180a10630
   180590b67:	48 8d 8d c0 00 00 00 	lea    rcx,[rbp+0xc0]
   180590b6e:	ff 15 44 68 42 00    	call   QWORD PTR [rip+0x426844]        # 0x1809b73b8
   180590b74:	48 8b d0             	mov    rdx,rax
   180590b77:	48 8d 4d c0          	lea    rcx,[rbp-0x40]
   180590b7b:	ff 15 97 67 42 00    	call   QWORD PTR [rip+0x426797]        # 0x1809b7318
   180590b81:	48 8d 8d c0 00 00 00 	lea    rcx,[rbp+0xc0]
   180590b88:	ff 15 92 67 42 00    	call   QWORD PTR [rip+0x426792]        # 0x1809b7320
   180590b8e:	48 8b 54 24 30       	mov    rdx,QWORD PTR [rsp+0x30]
   180590b93:	48 8d 4d c0          	lea    rcx,[rbp-0x40]
   180590b97:	ff 15 6b 67 42 00    	call   QWORD PTR [rip+0x42676b]        # 0x1809b7308
   180590b9d:	48 8d 4c 24 70       	lea    rcx,[rsp+0x70]
   180590ba2:	e8 b9 43 41 00       	call   0x1809a4f60
   180590ba7:	44 8b c6             	mov    r8d,esi
   180590baa:	48 8d 4c 24 70       	lea    rcx,[rsp+0x70]
   180590baf:	ba 78 1d e8 b7       	mov    edx,0xb7e81d78
   180590bb4:	e8 37 59 41 00       	call   0x1809a64f0
   180590bb9:	4c 8b 44 24 30       	mov    r8,QWORD PTR [rsp+0x30]
   180590bbe:	48 8d 4c 24 70       	lea    rcx,[rsp+0x70]
   180590bc3:	ba 40 14 3a ca       	mov    edx,0xca3a1440
   180590bc8:	e8 73 62 41 00       	call   0x1809a6e40
   180590bcd:	49 8b cd             	mov    rcx,r13
   180590bd0:	c7 44 24 58 9d 6b e9 	mov    DWORD PTR [rsp+0x58],0xd3e96b9d
   180590bd7:	d3 
   180590bd8:	89 7c 24 5c          	mov    DWORD PTR [rsp+0x5c],edi
   180590bdc:	e8 df f6 ff ff       	call   0x1805902c0
   180590be1:	4c 8b 44 24 58       	mov    r8,QWORD PTR [rsp+0x58]
   180590be6:	4c 8d 4d 40          	lea    r9,[rbp+0x40]
   180590bea:	8b d0                	mov    edx,eax
   180590bec:	48 8d 8d 70 01 00 00 	lea    rcx,[rbp+0x170]
   180590bf3:	48 8d 44 24 70       	lea    rax,[rsp+0x70]
   180590bf8:	48 89 44 24 28       	mov    QWORD PTR [rsp+0x28],rax
   180590bfd:	48 8d 45 c0          	lea    rax,[rbp-0x40]
   180590c01:	48 89 44 24 20       	mov    QWORD PTR [rsp+0x20],rax
   180590c06:	e8 95 bc 06 00       	call   0x1805fc8a0
   180590c0b:	48 8d 4c 24 70       	lea    rcx,[rsp+0x70]
   180590c10:	e8 cb 43 41 00       	call   0x1809a4fe0
   180590c15:	48 8d 4d c0          	lea    rcx,[rbp-0x40]
   180590c19:	ff 15 01 67 42 00    	call   QWORD PTR [rip+0x426701]        # 0x1809b7320
   180590c1f:	48 8d 4d 40          	lea    rcx,[rbp+0x40]
   180590c23:	ff 15 f7 66 42 00    	call   QWORD PTR [rip+0x4266f7]        # 0x1809b7320
   180590c29:	e9 e8 06 00 00       	jmp    0x180591316
   180590c2e:	48 8b 03             	mov    rax,QWORD PTR [rbx]
   180590c31:	ba d1 5f 99 f1       	mov    edx,0xf1995fd1
   180590c36:	48 8b cb             	mov    rcx,rbx
   180590c39:	40 32 ff             	xor    dil,dil
   180590c3c:	ff 50 60             	call   QWORD PTR [rax+0x60]
   180590c3f:	48 85 c0             	test   rax,rax
   180590c42:	74 0b                	je     0x180590c4f
   180590c44:	48 8b c8             	mov    rcx,rax
   180590c47:	e8 24 a3 aa ff       	call   0x18003af70
   180590c4c:	0f b6 f8             	movzx  edi,al
   180590c4f:	44 0f b6 cf          	movzx  r9d,dil
   180590c53:	45 33 c0             	xor    r8d,r8d
   180590c56:	e9 0f fd ff ff       	jmp    0x18059096a
   180590c5b:	81 ff f6 74 a2 54    	cmp    edi,0x54a274f6
   180590c61:	74 5e                	je     0x180590cc1
   180590c63:	81 ff 18 c2 12 6f    	cmp    edi,0x6f12c218
   180590c69:	74 1d                	je     0x180590c88
   180590c6b:	81 ff fe a4 ab 79    	cmp    edi,0x79aba4fe
   180590c71:	0f 85 4a 08 00 00    	jne    0x1805914c1
   180590c77:	49 83 c9 ff          	or     r9,0xffffffffffffffff
   180590c7b:	c6 44 24 20 00       	mov    BYTE PTR [rsp+0x20],0x0
   180590c80:	45 33 c0             	xor    r8d,r8d
   180590c83:	e9 22 08 00 00       	jmp    0x1805914aa
   180590c88:	48 8b 03             	mov    rax,QWORD PTR [rbx]
   180590c8b:	ba d8 b7 c5 86       	mov    edx,0x86c5b7d8
   180590c90:	48 8b cb             	mov    rcx,rbx
   180590c93:	40 32 ff             	xor    dil,dil
   180590c96:	ff 50 60             	call   QWORD PTR [rax+0x60]
   180590c99:	48 85 c0             	test   rax,rax
   180590c9c:	74 0b                	je     0x180590ca9
   180590c9e:	48 8b c8             	mov    rcx,rax
   180590ca1:	e8 ca a2 aa ff       	call   0x18003af70
   180590ca6:	0f b6 f8             	movzx  edi,al
   180590ca9:	40 0f b6 d7          	movzx  edx,dil
   180590cad:	48 8d 4c 24 30       	lea    rcx,[rsp+0x30]
   180590cb2:	e8 79 50 aa ff       	call   0x180035d30
   180590cb7:	48 8d 4c 24 30       	lea    rcx,[rsp+0x30]
   180590cbc:	e9 fb 07 00 00       	jmp    0x1805914bc
   180590cc1:	48 8d 4c 24 70       	lea    rcx,[rsp+0x70]
   180590cc6:	e8 95 42 41 00       	call   0x1809a4f60
   180590ccb:	48 8d 4d c0          	lea    rcx,[rbp-0x40]
   180590ccf:	ff 15 cb 66 42 00    	call   QWORD PTR [rip+0x4266cb]        # 0x1809b73a0
   180590cd5:	48 8d 15 44 f8 47 00 	lea    rdx,[rip+0x47f844]        # 0x180a10520
   180590cdc:	48 8d 8d c0 00 00 00 	lea    rcx,[rbp+0xc0]
   180590ce3:	ff 15 cf 66 42 00    	call   QWORD PTR [rip+0x4266cf]        # 0x1809b73b8
   180590ce9:	48 8b d0             	mov    rdx,rax
   180590cec:	48 8d 4d c0          	lea    rcx,[rbp-0x40]
   180590cf0:	ff 15 22 66 42 00    	call   QWORD PTR [rip+0x426622]        # 0x1809b7318
   180590cf6:	48 8d 8d c0 00 00 00 	lea    rcx,[rbp+0xc0]
   180590cfd:	ff 15 1d 66 42 00    	call   QWORD PTR [rip+0x42661d]        # 0x1809b7320
   180590d03:	48 8d 4d 40          	lea    rcx,[rbp+0x40]
   180590d07:	ff 15 93 66 42 00    	call   QWORD PTR [rip+0x426693]        # 0x1809b73a0
   180590d0d:	48 8d 15 2c f8 47 00 	lea    rdx,[rip+0x47f82c]        # 0x180a10540
   180590d14:	48 8d 8d c0 00 00 00 	lea    rcx,[rbp+0xc0]
   180590d1b:	ff 15 97 66 42 00    	call   QWORD PTR [rip+0x426697]        # 0x1809b73b8
   180590d21:	48 8b d0             	mov    rdx,rax
   180590d24:	48 8d 4d 40          	lea    rcx,[rbp+0x40]
   180590d28:	ff 15 ea 65 42 00    	call   QWORD PTR [rip+0x4265ea]        # 0x1809b7318
   180590d2e:	48 8d 8d c0 00 00 00 	lea    rcx,[rbp+0xc0]
   180590d35:	ff 15 e5 65 42 00    	call   QWORD PTR [rip+0x4265e5]        # 0x1809b7320
   180590d3b:	33 ff                	xor    edi,edi
   180590d3d:	c7 44 24 58 35 9d 29 	mov    DWORD PTR [rsp+0x58],0xb6299d35
   180590d44:	b6 
   180590d45:	89 7c 24 5c          	mov    DWORD PTR [rsp+0x5c],edi
   180590d49:	e9 b8 00 00 00       	jmp    0x180590e06
   180590d4e:	41 83 fc ff          	cmp    r12d,0xffffffff
   180590d52:	0f 84 69 07 00 00    	je     0x1805914c1
   180590d58:	48 8d 4c 24 70       	lea    rcx,[rsp+0x70]
   180590d5d:	e8 fe 41 41 00       	call   0x1809a4f60
   180590d62:	49 8b 4d 08          	mov    rcx,QWORD PTR [r13+0x8]
   180590d66:	41 8b d4             	mov    edx,r12d
   180590d69:	48 81 c1 80 0d 00 00 	add    rcx,0xd80
   180590d70:	e8 eb 3f bd ff       	call   0x180164d60
   180590d75:	48 8d 4d c0          	lea    rcx,[rbp-0x40]
   180590d79:	48 8b d8             	mov    rbx,rax
   180590d7c:	ff 15 1e 66 42 00    	call   QWORD PTR [rip+0x42661e]        # 0x1809b73a0
   180590d82:	48 8d 15 df f7 47 00 	lea    rdx,[rip+0x47f7df]        # 0x180a10568
   180590d89:	48 8d 8d c0 00 00 00 	lea    rcx,[rbp+0xc0]
   180590d90:	ff 15 22 66 42 00    	call   QWORD PTR [rip+0x426622]        # 0x1809b73b8
   180590d96:	48 8b d0             	mov    rdx,rax
   180590d99:	48 8d 4d c0          	lea    rcx,[rbp-0x40]
   180590d9d:	ff 15 75 65 42 00    	call   QWORD PTR [rip+0x426575]        # 0x1809b7318
   180590da3:	48 8d 8d c0 00 00 00 	lea    rcx,[rbp+0xc0]
   180590daa:	ff 15 70 65 42 00    	call   QWORD PTR [rip+0x426570]        # 0x1809b7320
   180590db0:	48 8d 4d 40          	lea    rcx,[rbp+0x40]
   180590db4:	ff 15 e6 65 42 00    	call   QWORD PTR [rip+0x4265e6]        # 0x1809b73a0
   180590dba:	48 8d 15 c7 f7 47 00 	lea    rdx,[rip+0x47f7c7]        # 0x180a10588
   180590dc1:	48 8d 8d c0 00 00 00 	lea    rcx,[rbp+0xc0]
   180590dc8:	ff 15 ea 65 42 00    	call   QWORD PTR [rip+0x4265ea]        # 0x1809b73b8
   180590dce:	48 8b d0             	mov    rdx,rax
   180590dd1:	48 8d 4d 40          	lea    rcx,[rbp+0x40]
   180590dd5:	ff 15 3d 65 42 00    	call   QWORD PTR [rip+0x42653d]        # 0x1809b7318
   180590ddb:	48 8d 8d c0 00 00 00 	lea    rcx,[rbp+0xc0]
   180590de2:	ff 15 38 65 42 00    	call   QWORD PTR [rip+0x426538]        # 0x1809b7320
   180590de8:	48 8b cb             	mov    rcx,rbx
   180590deb:	e8 40 93 bd ff       	call   0x18016a130
   180590df0:	48 8b d0             	mov    rdx,rax
   180590df3:	48 8d 4d 40          	lea    rcx,[rbp+0x40]
   180590df7:	ff 15 0b 65 42 00    	call   QWORD PTR [rip+0x42650b]        # 0x1809b7308
   180590dfd:	48 c7 44 24 58 8c b5 	mov    QWORD PTR [rsp+0x58],0x48f3b58c
   180590e04:	f3 48 
   180590e06:	49 8b cd             	mov    rcx,r13
   180590e09:	e8 b2 f4 ff ff       	call   0x1805902c0
   180590e0e:	4c 8b 44 24 58       	mov    r8,QWORD PTR [rsp+0x58]
   180590e13:	4c 8d 4d c0          	lea    r9,[rbp-0x40]
   180590e17:	8b d0                	mov    edx,eax
   180590e19:	48 8d 8d 70 01 00 00 	lea    rcx,[rbp+0x170]
   180590e20:	48 8d 44 24 70       	lea    rax,[rsp+0x70]
   180590e25:	48 89 44 24 28       	mov    QWORD PTR [rsp+0x28],rax
   180590e2a:	48 8d 45 40          	lea    rax,[rbp+0x40]
   180590e2e:	48 89 44 24 20       	mov    QWORD PTR [rsp+0x20],rax
   180590e33:	e8 68 ba 06 00       	call   0x1805fc8a0
   180590e38:	48 8d 4d 40          	lea    rcx,[rbp+0x40]
   180590e3c:	ff 15 de 64 42 00    	call   QWORD PTR [rip+0x4264de]        # 0x1809b7320
   180590e42:	48 8d 4d c0          	lea    rcx,[rbp-0x40]
   180590e46:	ff 15 d4 64 42 00    	call   QWORD PTR [rip+0x4264d4]        # 0x1809b7320
   180590e4c:	48 8d 4c 24 70       	lea    rcx,[rsp+0x70]
   180590e51:	e8 8a 41 41 00       	call   0x1809a4fe0
   180590e56:	e9 66 06 00 00       	jmp    0x1805914c1
   180590e5b:	81 ff 63 d8 82 c3    	cmp    edi,0xc382d863
   180590e61:	0f 87 e6 04 00 00    	ja     0x18059134d
   180590e67:	0f 84 6c 02 00 00    	je     0x1805910d9
   180590e6d:	81 ff ca 81 d7 81    	cmp    edi,0x81d781ca
   180590e73:	0f 84 ba 00 00 00    	je     0x180590f33
   180590e79:	81 ff fb b9 86 b1    	cmp    edi,0xb186b9fb
   180590e7f:	74 38                	je     0x180590eb9
   180590e81:	81 ff 3d 64 ab bd    	cmp    edi,0xbdab643d
   180590e87:	0f 85 34 06 00 00    	jne    0x1805914c1
   180590e8d:	ba 54 7e 0b d6       	mov    edx,0xd60b7e54
   180590e92:	41 8b cf             	mov    ecx,r15d
   180590e95:	e8 26 ca ac ff       	call   0x18005d8c0
   180590e9a:	84 c0                	test   al,al
   180590e9c:	0f 84 1f 06 00 00    	je     0x1805914c1
   180590ea2:	41 8b d7             	mov    edx,r15d
   180590ea5:	48 8d 4c 24 30       	lea    rcx,[rsp+0x30]
   180590eaa:	e8 01 90 aa ff       	call   0x180039eb0
   180590eaf:	48 8d 4c 24 30       	lea    rcx,[rsp+0x30]
   180590eb4:	e9 03 06 00 00       	jmp    0x1805914bc
   180590eb9:	48 8b 03             	mov    rax,QWORD PTR [rbx]
   180590ebc:	ba 1d 95 af c4       	mov    edx,0xc4af951d
   180590ec1:	48 8b cb             	mov    rcx,rbx
   180590ec4:	bf 01 00 00 00       	mov    edi,0x1
   180590ec9:	40 32 f6             	xor    sil,sil
   180590ecc:	ff 50 60             	call   QWORD PTR [rax+0x60]
   180590ecf:	48 85 c0             	test   rax,rax
   180590ed2:	74 0b                	je     0x180590edf
   180590ed4:	48 8b c8             	mov    rcx,rax
   180590ed7:	e8 94 a0 aa ff       	call   0x18003af70
   180590edc:	0f b6 f0             	movzx  esi,al
   180590edf:	4c 8b 03             	mov    r8,QWORD PTR [rbx]
   180590ee2:	ba f9 ac 0e ef       	mov    edx,0xef0eacf9
   180590ee7:	48 8b cb             	mov    rcx,rbx
   180590eea:	41 ff 50 60          	call   QWORD PTR [r8+0x60]
   180590eee:	48 85 c0             	test   rax,rax
   180590ef1:	74 27                	je     0x180590f1a
   180590ef3:	8b 48 14             	mov    ecx,DWORD PTR [rax+0x14]
   180590ef6:	c1 e9 1d             	shr    ecx,0x1d
   180590ef9:	40 84 cf             	test   dil,cl
   180590efc:	75 1a                	jne    0x180590f18
   180590efe:	0f b7 08             	movzx  ecx,WORD PTR [rax]
   180590f01:	66 83 f9 15          	cmp    cx,0x15
   180590f05:	77 11                	ja     0x180590f18
   180590f07:	41 be 2e 00 30 00    	mov    r14d,0x30002e
   180590f0d:	41 0f a3 ce          	bt     r14d,ecx
   180590f11:	73 05                	jae    0x180590f18
   180590f13:	8b 78 08             	mov    edi,DWORD PTR [rax+0x8]
   180590f16:	eb 02                	jmp    0x180590f1a
   180590f18:	33 ff                	xor    edi,edi
   180590f1a:	44 8b cf             	mov    r9d,edi
   180590f1d:	48 8d 4c 24 30       	lea    rcx,[rsp+0x30]
   180590f22:	44 0f b6 c6          	movzx  r8d,sil
   180590f26:	41 8b d7             	mov    edx,r15d
   180590f29:	e8 22 51 aa ff       	call   0x180036050
   180590f2e:	e9 84 05 00 00       	jmp    0x1805914b7
   180590f33:	48 8b 03             	mov    rax,QWORD PTR [rbx]
   180590f36:	33 ff                	xor    edi,edi
   180590f38:	ba 32 b7 4f d0       	mov    edx,0xd04fb732
   180590f3d:	48 8b cb             	mov    rcx,rbx
   180590f40:	44 8b ff             	mov    r15d,edi
   180590f43:	8b f7                	mov    esi,edi
   180590f45:	ff 50 60             	call   QWORD PTR [rax+0x60]
   180590f48:	48 8b d0             	mov    rdx,rax
   180590f4b:	41 be 2e 00 30 00    	mov    r14d,0x30002e
   180590f51:	48 85 c0             	test   rax,rax
   180590f54:	74 1f                	je     0x180590f75
   180590f56:	8b 48 14             	mov    ecx,DWORD PTR [rax+0x14]
   180590f59:	c1 e9 1d             	shr    ecx,0x1d
   180590f5c:	f6 c1 01             	test   cl,0x1
   180590f5f:	75 14                	jne    0x180590f75
   180590f61:	0f b7 00             	movzx  eax,WORD PTR [rax]
   180590f64:	66 83 f8 15          	cmp    ax,0x15
   180590f68:	77 0b                	ja     0x180590f75
   180590f6a:	98                   	cwde
   180590f6b:	41 0f a3 c6          	bt     r14d,eax
   180590f6f:	73 04                	jae    0x180590f75
   180590f71:	44 8b 7a 08          	mov    r15d,DWORD PTR [rdx+0x8]
   180590f75:	48 8b 03             	mov    rax,QWORD PTR [rbx]
   180590f78:	ba 06 ca 89 97       	mov    edx,0x9789ca06
   180590f7d:	48 8b cb             	mov    rcx,rbx
   180590f80:	ff 50 60             	call   QWORD PTR [rax+0x60]
   180590f83:	48 85 c0             	test   rax,rax
   180590f86:	74 20                	je     0x180590fa8
   180590f88:	8b 48 14             	mov    ecx,DWORD PTR [rax+0x14]
   180590f8b:	c1 e9 1d             	shr    ecx,0x1d
   180590f8e:	f6 c1 01             	test   cl,0x1
   180590f91:	75 15                	jne    0x180590fa8
   180590f93:	0f b7 08             	movzx  ecx,WORD PTR [rax]
   180590f96:	66 83 f9 15          	cmp    cx,0x15
   180590f9a:	77 0c                	ja     0x180590fa8
   180590f9c:	0f bf c9             	movsx  ecx,cx
   180590f9f:	41 0f a3 ce          	bt     r14d,ecx
   180590fa3:	73 03                	jae    0x180590fa8
   180590fa5:	8b 70 08             	mov    esi,DWORD PTR [rax+0x8]
   180590fa8:	49 8b 4d 08          	mov    rcx,QWORD PTR [r13+0x8]
   180590fac:	41 8b d4             	mov    edx,r12d
   180590faf:	48 81 c1 80 0d 00 00 	add    rcx,0xd80
   180590fb6:	e8 a5 3d bd ff       	call   0x180164d60
   180590fbb:	48 8d 4c 24 70       	lea    rcx,[rsp+0x70]
   180590fc0:	48 8b d8             	mov    rbx,rax
   180590fc3:	e8 98 3f 41 00       	call   0x1809a4f60
   180590fc8:	48 8d 4d 40          	lea    rcx,[rbp+0x40]
   180590fcc:	ff 15 ce 63 42 00    	call   QWORD PTR [rip+0x4263ce]        # 0x1809b73a0
   180590fd2:	48 8d 15 f7 f4 47 00 	lea    rdx,[rip+0x47f4f7]        # 0x180a104d0
   180590fd9:	48 8d 8d c0 00 00 00 	lea    rcx,[rbp+0xc0]
   180590fe0:	ff 15 d2 63 42 00    	call   QWORD PTR [rip+0x4263d2]        # 0x1809b73b8
   180590fe6:	48 8b d0             	mov    rdx,rax
   180590fe9:	48 8d 4d 40          	lea    rcx,[rbp+0x40]
   180590fed:	ff 15 25 63 42 00    	call   QWORD PTR [rip+0x426325]        # 0x1809b7318
   180590ff3:	48 8d 8d c0 00 00 00 	lea    rcx,[rbp+0xc0]
   180590ffa:	ff 15 20 63 42 00    	call   QWORD PTR [rip+0x426320]        # 0x1809b7320
   180591000:	48 8b cb             	mov    rcx,rbx
   180591003:	e8 28 91 bd ff       	call   0x18016a130
   180591008:	48 8b d0             	mov    rdx,rax
   18059100b:	48 8d 4d 40          	lea    rcx,[rbp+0x40]
   18059100f:	ff 15 f3 62 42 00    	call   QWORD PTR [rip+0x4262f3]        # 0x1809b7308
   180591015:	48 8d 4d c0          	lea    rcx,[rbp-0x40]
   180591019:	ff 15 81 63 42 00    	call   QWORD PTR [rip+0x426381]        # 0x1809b73a0
   18059101f:	48 8d 15 d2 f4 47 00 	lea    rdx,[rip+0x47f4d2]        # 0x180a104f8
   180591026:	48 8d 8d c0 00 00 00 	lea    rcx,[rbp+0xc0]
   18059102d:	ff 15 85 63 42 00    	call   QWORD PTR [rip+0x426385]        # 0x1809b73b8
   180591033:	48 8b d0             	mov    rdx,rax
   180591036:	48 8d 4d c0          	lea    rcx,[rbp-0x40]
   18059103a:	ff 15 d8 62 42 00    	call   QWORD PTR [rip+0x4262d8]        # 0x1809b7318
   180591040:	48 8d 8d c0 00 00 00 	lea    rcx,[rbp+0xc0]
   180591047:	ff 15 d3 62 42 00    	call   QWORD PTR [rip+0x4262d3]        # 0x1809b7320
   18059104d:	48 8b cb             	mov    rcx,rbx
   180591050:	e8 db 90 bd ff       	call   0x18016a130
   180591055:	48 8b d0             	mov    rdx,rax
   180591058:	48 8d 4d c0          	lea    rcx,[rbp-0x40]
   18059105c:	ff 15 a6 62 42 00    	call   QWORD PTR [rip+0x4262a6]        # 0x1809b7308
   180591062:	41 8b d7             	mov    edx,r15d
   180591065:	48 8d 4d c0          	lea    rcx,[rbp-0x40]
   180591069:	ff 15 21 63 42 00    	call   QWORD PTR [rip+0x426321]        # 0x1809b7390
   18059106f:	8b d6                	mov    edx,esi
   180591071:	48 8d 4d c0          	lea    rcx,[rbp-0x40]
   180591075:	ff 15 15 63 42 00    	call   QWORD PTR [rip+0x426315]        # 0x1809b7390
   18059107b:	49 8b cd             	mov    rcx,r13
   18059107e:	48 c7 44 24 58 e1 1c 	mov    QWORD PTR [rsp+0x58],0x22b21ce1
   180591085:	b2 22 
   180591087:	e8 34 f2 ff ff       	call   0x1805902c0
   18059108c:	4c 8b 44 24 58       	mov    r8,QWORD PTR [rsp+0x58]
   180591091:	4c 8d 4d 40          	lea    r9,[rbp+0x40]
   180591095:	8b d0                	mov    edx,eax
   180591097:	48 8d 8d 70 01 00 00 	lea    rcx,[rbp+0x170]
   18059109e:	48 8d 44 24 70       	lea    rax,[rsp+0x70]
   1805910a3:	48 89 44 24 28       	mov    QWORD PTR [rsp+0x28],rax
   1805910a8:	48 8d 45 c0          	lea    rax,[rbp-0x40]
   1805910ac:	48 89 44 24 20       	mov    QWORD PTR [rsp+0x20],rax
   1805910b1:	e8 ea b7 06 00       	call   0x1805fc8a0
   1805910b6:	48 8d 4d c0          	lea    rcx,[rbp-0x40]
   1805910ba:	ff 15 60 62 42 00    	call   QWORD PTR [rip+0x426260]        # 0x1809b7320
   1805910c0:	48 8d 4d 40          	lea    rcx,[rbp+0x40]
   1805910c4:	ff 15 56 62 42 00    	call   QWORD PTR [rip+0x426256]        # 0x1809b7320
   1805910ca:	48 8d 4c 24 70       	lea    rcx,[rsp+0x70]
   1805910cf:	e8 0c 3f 41 00       	call   0x1809a4fe0
   1805910d4:	e9 e8 03 00 00       	jmp    0x1805914c1
   1805910d9:	48 8d 05 1c f6 5f 00 	lea    rax,[rip+0x5ff61c]        # 0x180b906fc
   1805910e0:	ba 3e dc ca cf       	mov    edx,0xcfcadc3e
   1805910e5:	48 89 44 24 30       	mov    QWORD PTR [rsp+0x30],rax
   1805910ea:	48 8b cb             	mov    rcx,rbx
   1805910ed:	48 89 44 24 38       	mov    QWORD PTR [rsp+0x38],rax
   1805910f2:	48 8d 05 04 f6 5f 00 	lea    rax,[rip+0x5ff604]        # 0x180b906fd
   1805910f9:	48 89 44 24 40       	mov    QWORD PTR [rsp+0x40],rax
   1805910fe:	48 8b 03             	mov    rax,QWORD PTR [rbx]
   180591101:	ff 50 60             	call   QWORD PTR [rax+0x60]
   180591104:	48 83 ce ff          	or     rsi,0xffffffffffffffff
   180591108:	48 85 c0             	test   rax,rax
   18059110b:	0f 84 85 00 00 00    	je     0x180591196
   180591111:	8b 48 14             	mov    ecx,DWORD PTR [rax+0x14]
   180591114:	c1 e9 1d             	shr    ecx,0x1d
   180591117:	f6 c1 01             	test   cl,0x1
   18059111a:	74 7a                	je     0x180591196
   18059111c:	66 83 38 05          	cmp    WORD PTR [rax],0x5
   180591120:	75 74                	jne    0x180591196
   180591122:	4c 8b 70 08          	mov    r14,QWORD PTR [rax+0x8]
   180591126:	4d 85 f6             	test   r14,r14
   180591129:	74 6b                	je     0x180591196
   18059112b:	48 8b fe             	mov    rdi,rsi
   18059112e:	66 90                	xchg   ax,ax
   180591130:	48 ff c7             	inc    rdi
   180591133:	41 80 3c 3e 00       	cmp    BYTE PTR [r14+rdi*1],0x0
   180591138:	75 f6                	jne    0x180591130
   18059113a:	4c 8b 44 24 38       	mov    r8,QWORD PTR [rsp+0x38]
   18059113f:	49 03 fe             	add    rdi,r14
   180591142:	48 8b 4c 24 30       	mov    rcx,QWORD PTR [rsp+0x30]
   180591147:	4c 8b ff             	mov    r15,rdi
   18059114a:	41 8b c0             	mov    eax,r8d
   18059114d:	4d 2b fe             	sub    r15,r14
   180591150:	2b c1                	sub    eax,ecx
   180591152:	49 8b d6             	mov    rdx,r14
   180591155:	44 3b f8             	cmp    r15d,eax
   180591158:	77 79                	ja     0x1805911d3
   18059115a:	4d 8b c7             	mov    r8,r15
   18059115d:	ff 15 9d 64 42 00    	call   QWORD PTR [rip+0x42649d]        # 0x1809b7600
   180591163:	4c 8b 74 24 30       	mov    r14,QWORD PTR [rsp+0x30]
   180591168:	4c 8b 6c 24 38       	mov    r13,QWORD PTR [rsp+0x38]
   18059116d:	4d 03 f7             	add    r14,r15
   180591170:	4d 3b f5             	cmp    r14,r13
   180591173:	74 1a                	je     0x18059118f
   180591175:	41 b8 01 00 00 00    	mov    r8d,0x1
   18059117b:	49 8b d5             	mov    rdx,r13
   18059117e:	49 8b ce             	mov    rcx,r14
   180591181:	ff 15 79 64 42 00    	call   QWORD PTR [rip+0x426479]        # 0x1809b7600
   180591187:	4d 2b f5             	sub    r14,r13
   18059118a:	4c 01 74 24 38       	add    QWORD PTR [rsp+0x38],r14
   18059118f:	4c 8b ad 70 01 00 00 	mov    r13,QWORD PTR [rbp+0x170]
   180591196:	48 8b 03             	mov    rax,QWORD PTR [rbx]
   180591199:	ba cb a5 ed f4       	mov    edx,0xf4eda5cb
   18059119e:	48 8b cb             	mov    rcx,rbx
   1805911a1:	ff 50 60             	call   QWORD PTR [rax+0x60]
   1805911a4:	33 ff                	xor    edi,edi
   1805911a6:	48 8b d0             	mov    rdx,rax
   1805911a9:	48 85 c0             	test   rax,rax
   1805911ac:	74 4a                	je     0x1805911f8
   1805911ae:	8b 48 14             	mov    ecx,DWORD PTR [rax+0x14]
   1805911b1:	c1 e9 1d             	shr    ecx,0x1d
   1805911b4:	f6 c1 01             	test   cl,0x1
   1805911b7:	75 3d                	jne    0x1805911f6
   1805911b9:	0f b7 00             	movzx  eax,WORD PTR [rax]
   1805911bc:	66 83 f8 15          	cmp    ax,0x15
   1805911c0:	77 34                	ja     0x1805911f6
   1805911c2:	41 be 2e 00 30 00    	mov    r14d,0x30002e
   1805911c8:	41 0f a3 c6          	bt     r14d,eax
   1805911cc:	73 28                	jae    0x1805911f6
   1805911ce:	8b 72 08             	mov    esi,DWORD PTR [rdx+0x8]
   1805911d1:	eb 25                	jmp    0x1805911f8
   1805911d3:	4c 2b c1             	sub    r8,rcx
   1805911d6:	ff 15 24 64 42 00    	call   QWORD PTR [rip+0x426424]        # 0x1809b7600
   1805911dc:	8b 54 24 38          	mov    edx,DWORD PTR [rsp+0x38]
   1805911e0:	48 8d 4c 24 30       	lea    rcx,[rsp+0x30]
   1805911e5:	2b 54 24 30          	sub    edx,DWORD PTR [rsp+0x30]
   1805911e9:	4c 8b c7             	mov    r8,rdi
   1805911ec:	49 03 d6             	add    rdx,r14
   1805911ef:	e8 8c 38 a8 ff       	call   0x180014a80
   1805911f4:	eb a0                	jmp    0x180591196
   1805911f6:	8b f7                	mov    esi,edi
   1805911f8:	48 8d 4c 24 70       	lea    rcx,[rsp+0x70]
   1805911fd:	e8 5e 3d 41 00       	call   0x1809a4f60
   180591202:	44 8b c6             	mov    r8d,esi
   180591205:	48 8d 4c 24 70       	lea    rcx,[rsp+0x70]
   18059120a:	ba 78 1d e8 b7       	mov    edx,0xb7e81d78
   18059120f:	e8 dc 52 41 00       	call   0x1809a64f0
   180591214:	4c 8b 44 24 30       	mov    r8,QWORD PTR [rsp+0x30]
   180591219:	48 8d 4c 24 70       	lea    rcx,[rsp+0x70]
   18059121e:	ba 40 14 3a ca       	mov    edx,0xca3a1440
   180591223:	e8 18 5c 41 00       	call   0x1809a6e40
   180591228:	49 8b 4d 08          	mov    rcx,QWORD PTR [r13+0x8]
   18059122c:	41 8b d4             	mov    edx,r12d
   18059122f:	48 81 c1 80 0d 00 00 	add    rcx,0xd80
   180591236:	e8 25 3b bd ff       	call   0x180164d60
   18059123b:	48 8d 4d c0          	lea    rcx,[rbp-0x40]
   18059123f:	ff 15 5b 61 42 00    	call   QWORD PTR [rip+0x42615b]        # 0x1809b73a0
   180591245:	48 8d 15 14 f4 47 00 	lea    rdx,[rip+0x47f414]        # 0x180a10660
   18059124c:	48 8d 8d c0 00 00 00 	lea    rcx,[rbp+0xc0]
   180591253:	ff 15 5f 61 42 00    	call   QWORD PTR [rip+0x42615f]        # 0x1809b73b8
   180591259:	48 8b d0             	mov    rdx,rax
   18059125c:	48 8d 4d c0          	lea    rcx,[rbp-0x40]
   180591260:	ff 15 b2 60 42 00    	call   QWORD PTR [rip+0x4260b2]        # 0x1809b7318
   180591266:	48 8d 8d c0 00 00 00 	lea    rcx,[rbp+0xc0]
   18059126d:	ff 15 ad 60 42 00    	call   QWORD PTR [rip+0x4260ad]        # 0x1809b7320
   180591273:	48 8d 4d 40          	lea    rcx,[rbp+0x40]
   180591277:	ff 15 23 61 42 00    	call   QWORD PTR [rip+0x426123]        # 0x1809b73a0
   18059127d:	48 8d 15 0c f4 47 00 	lea    rdx,[rip+0x47f40c]        # 0x180a10690
   180591284:	48 8d 8d c0 00 00 00 	lea    rcx,[rbp+0xc0]
   18059128b:	ff 15 27 61 42 00    	call   QWORD PTR [rip+0x426127]        # 0x1809b73b8
   180591291:	48 8b d0             	mov    rdx,rax
   180591294:	48 8d 4d 40          	lea    rcx,[rbp+0x40]
   180591298:	ff 15 7a 60 42 00    	call   QWORD PTR [rip+0x42607a]        # 0x1809b7318
   18059129e:	48 8d 8d c0 00 00 00 	lea    rcx,[rbp+0xc0]
   1805912a5:	ff 15 75 60 42 00    	call   QWORD PTR [rip+0x426075]        # 0x1809b7320
   1805912ab:	48 8b 54 24 30       	mov    rdx,QWORD PTR [rsp+0x30]
   1805912b0:	48 8d 4d 40          	lea    rcx,[rbp+0x40]
   1805912b4:	ff 15 4e 60 42 00    	call   QWORD PTR [rip+0x42604e]        # 0x1809b7308
   1805912ba:	49 8b cd             	mov    rcx,r13
   1805912bd:	c7 44 24 58 f9 a5 89 	mov    DWORD PTR [rsp+0x58],0xf089a5f9
   1805912c4:	f0 
   1805912c5:	89 7c 24 5c          	mov    DWORD PTR [rsp+0x5c],edi
   1805912c9:	e8 f2 ef ff ff       	call   0x1805902c0
   1805912ce:	4c 8b 44 24 58       	mov    r8,QWORD PTR [rsp+0x58]
   1805912d3:	4c 8d 4d c0          	lea    r9,[rbp-0x40]
   1805912d7:	8b d0                	mov    edx,eax
   1805912d9:	48 8d 8d 70 01 00 00 	lea    rcx,[rbp+0x170]
   1805912e0:	48 8d 44 24 70       	lea    rax,[rsp+0x70]
   1805912e5:	48 89 44 24 28       	mov    QWORD PTR [rsp+0x28],rax
   1805912ea:	48 8d 45 40          	lea    rax,[rbp+0x40]
   1805912ee:	48 89 44 24 20       	mov    QWORD PTR [rsp+0x20],rax
   1805912f3:	e8 a8 b5 06 00       	call   0x1805fc8a0
   1805912f8:	48 8d 4d 40          	lea    rcx,[rbp+0x40]
   1805912fc:	ff 15 1e 60 42 00    	call   QWORD PTR [rip+0x42601e]        # 0x1809b7320
   180591302:	48 8d 4d c0          	lea    rcx,[rbp-0x40]
   180591306:	ff 15 14 60 42 00    	call   QWORD PTR [rip+0x426014]        # 0x1809b7320
   18059130c:	48 8d 4c 24 70       	lea    rcx,[rsp+0x70]
   180591311:	e8 ca 3c 41 00       	call   0x1809a4fe0
   180591316:	48 8b 4c 24 40       	mov    rcx,QWORD PTR [rsp+0x40]
   18059131b:	48 8b 54 24 30       	mov    rdx,QWORD PTR [rsp+0x30]
   180591320:	48 8b c1             	mov    rax,rcx
   180591323:	48 2b c2             	sub    rax,rdx
   180591326:	48 83 f8 01          	cmp    rax,0x1
   18059132a:	0f 8e 91 01 00 00    	jle    0x1805914c1
   180591330:	48 85 d2             	test   rdx,rdx
   180591333:	0f 84 88 01 00 00    	je     0x1805914c1
   180591339:	2b ca                	sub    ecx,edx
   18059133b:	44 8b c1             	mov    r8d,ecx
   18059133e:	48 8d 4c 24 48       	lea    rcx,[rsp+0x48]
   180591343:	e8 d8 5a aa ff       	call   0x180036e20
   180591348:	e9 74 01 00 00       	jmp    0x1805914c1
   18059134d:	81 ff da a4 02 c6    	cmp    edi,0xc602a4da
   180591353:	0f 84 9d 00 00 00    	je     0x1805913f6
   180591359:	81 ff 81 e2 63 dd    	cmp    edi,0xdd63e281
   18059135f:	74 23                	je     0x180591384
   180591361:	81 ff 7e 49 4e f1    	cmp    edi,0xf14e497e
   180591367:	0f 85 54 01 00 00    	jne    0x1805914c1
   18059136d:	41 8b d7             	mov    edx,r15d
   180591370:	48 8d 4c 24 30       	lea    rcx,[rsp+0x30]
   180591375:	e8 26 7a aa ff       	call   0x180038da0
   18059137a:	48 8d 4c 24 30       	lea    rcx,[rsp+0x30]
   18059137f:	e9 38 01 00 00       	jmp    0x1805914bc
   180591384:	48 8b 03             	mov    rax,QWORD PTR [rbx]
   180591387:	ba 93 49 d9 aa       	mov    edx,0xaad94993
   18059138c:	48 8b cb             	mov    rcx,rbx
   18059138f:	48 83 ce ff          	or     rsi,0xffffffffffffffff
   180591393:	ff 50 60             	call   QWORD PTR [rax+0x60]
   180591396:	48 85 c0             	test   rax,rax
   180591399:	74 41                	je     0x1805913dc
   18059139b:	8b 48 14             	mov    ecx,DWORD PTR [rax+0x14]
   18059139e:	c1 e9 1d             	shr    ecx,0x1d
   1805913a1:	f6 c1 01             	test   cl,0x1
   1805913a4:	75 32                	jne    0x1805913d8
   1805913a6:	0f b7 08             	movzx  ecx,WORD PTR [rax]
   1805913a9:	66 83 f9 15          	cmp    cx,0x15
   1805913ad:	77 29                	ja     0x1805913d8
   1805913af:	41 be 2e 00 30 00    	mov    r14d,0x30002e
   1805913b5:	41 0f a3 ce          	bt     r14d,ecx
   1805913b9:	73 1d                	jae    0x1805913d8
   1805913bb:	8b 70 08             	mov    esi,DWORD PTR [rax+0x8]
   1805913be:	48 8d 4c 24 30       	lea    rcx,[rsp+0x30]
   1805913c3:	44 8b c6             	mov    r8d,esi
   1805913c6:	41 8b d7             	mov    edx,r15d
   1805913c9:	e8 82 81 aa ff       	call   0x180039550
   1805913ce:	48 8d 4c 24 30       	lea    rcx,[rsp+0x30]
   1805913d3:	e9 e4 00 00 00       	jmp    0x1805914bc
   1805913d8:	33 ff                	xor    edi,edi
   1805913da:	8b f7                	mov    esi,edi
   1805913dc:	44 8b c6             	mov    r8d,esi
   1805913df:	48 8d 4c 24 30       	lea    rcx,[rsp+0x30]
   1805913e4:	41 8b d7             	mov    edx,r15d
   1805913e7:	e8 64 81 aa ff       	call   0x180039550
   1805913ec:	48 8d 4c 24 30       	lea    rcx,[rsp+0x30]
   1805913f1:	e9 c6 00 00 00       	jmp    0x1805914bc
   1805913f6:	48 8b 03             	mov    rax,QWORD PTR [rbx]
   1805913f9:	ba a2 44 c6 cb       	mov    edx,0xcbc644a2
   1805913fe:	48 8b cb             	mov    rcx,rbx
   180591401:	bf 02 00 00 00       	mov    edi,0x2
   180591406:	40 32 f6             	xor    sil,sil
   180591409:	ff 50 60             	call   QWORD PTR [rax+0x60]
   18059140c:	48 85 c0             	test   rax,rax
   18059140f:	74 27                	je     0x180591438
   180591411:	8b 48 14             	mov    ecx,DWORD PTR [rax+0x14]
   180591414:	c1 e9 1d             	shr    ecx,0x1d
   180591417:	f6 c1 01             	test   cl,0x1
   18059141a:	75 1a                	jne    0x180591436
   18059141c:	0f b7 08             	movzx  ecx,WORD PTR [rax]
   18059141f:	66 83 f9 15          	cmp    cx,0x15
   180591423:	77 11                	ja     0x180591436
   180591425:	41 be 2e 00 30 00    	mov    r14d,0x30002e
   18059142b:	41 0f a3 ce          	bt     r14d,ecx
   18059142f:	73 05                	jae    0x180591436
   180591431:	8b 78 08             	mov    edi,DWORD PTR [rax+0x8]
   180591434:	eb 02                	jmp    0x180591438
   180591436:	33 ff                	xor    edi,edi
   180591438:	48 8b 03             	mov    rax,QWORD PTR [rbx]
   18059143b:	ba 97 fb 26 47       	mov    edx,0x4726fb97
   180591440:	48 8b cb             	mov    rcx,rbx
   180591443:	ff 50 60             	call   QWORD PTR [rax+0x60]
   180591446:	48 8b d0             	mov    rdx,rax
   180591449:	48 85 c0             	test   rax,rax
   18059144c:	74 51                	je     0x18059149f
   18059144e:	8b 48 14             	mov    ecx,DWORD PTR [rax+0x14]
   180591451:	c1 e9 1d             	shr    ecx,0x1d
   180591454:	f6 c1 01             	test   cl,0x1
   180591457:	75 43                	jne    0x18059149c
   180591459:	44 0f b7 00          	movzx  r8d,WORD PTR [rax]
   18059145d:	b8 fb ff 00 00       	mov    eax,0xfffb
   180591462:	41 8d 48 ff          	lea    ecx,[r8-0x1]
   180591466:	66 85 c8             	test   ax,cx
   180591469:	74 2b                	je     0x180591496
   18059146b:	41 8d 40 fe          	lea    eax,[r8-0x2]
   18059146f:	66 83 f8 01          	cmp    ax,0x1
   180591473:	76 17                	jbe    0x18059148c
   180591475:	66 41 83 e8 14       	sub    r8w,0x14
   18059147a:	66 41 83 f8 01       	cmp    r8w,0x1
   18059147f:	77 1b                	ja     0x18059149c
   180591481:	48 83 7a 08 00       	cmp    QWORD PTR [rdx+0x8],0x0
   180591486:	40 0f 95 c6          	setne  sil
   18059148a:	eb 13                	jmp    0x18059149f
   18059148c:	83 7a 08 00          	cmp    DWORD PTR [rdx+0x8],0x0
   180591490:	40 0f 95 c6          	setne  sil
   180591494:	eb 09                	jmp    0x18059149f
   180591496:	0f b6 72 08          	movzx  esi,BYTE PTR [rdx+0x8]
   18059149a:	eb 03                	jmp    0x18059149f
   18059149c:	40 32 f6             	xor    sil,sil
   18059149f:	40 88 74 24 20       	mov    BYTE PTR [rsp+0x20],sil
   1805914a4:	44 8b cf             	mov    r9d,edi
   1805914a7:	41 b0 01             	mov    r8b,0x1
   1805914aa:	41 8b d7             	mov    edx,r15d
   1805914ad:	48 8d 4c 24 30       	lea    rcx,[rsp+0x30]
   1805914b2:	e8 29 7a aa ff       	call   0x180038ee0
   1805914b7:	48 8d 4c 24 30       	lea    rcx,[rsp+0x30]
   1805914bc:	e8 ef 51 aa ff       	call   0x1800366b0
   1805914c1:	4c 8b b4 24 80 02 00 	mov    r14,QWORD PTR [rsp+0x280]
   1805914c8:	00 
   1805914c9:	48 8b b4 24 78 02 00 	mov    rsi,QWORD PTR [rsp+0x278]
   1805914d0:	00 
   1805914d1:	4c 8b bc 24 88 02 00 	mov    r15,QWORD PTR [rsp+0x288]
   1805914d8:	00 
   1805914d9:	48 81 c4 40 02 00 00 	add    rsp,0x240
   1805914e0:	41 5d                	pop    r13
   1805914e2:	41 5c                	pop    r12
   1805914e4:	5f                   	pop    rdi
   1805914e5:	5b                   	pop    rbx
   1805914e6:	5d                   	pop    rbp
   1805914e7:	c3                   	ret

Exports::Context::RequestUnitOperation

/home/phulin/c6-assets/DLC/Expansion2/Binaries/Win64/GameCore_XP2_FinalRelease.dll:     file format pei-x86-64


Disassembly of section .text:

00000001805915d0 <.text+0x5905d0>:
   1805915d0:	41 8b c0             	mov    eax,r8d
   1805915d3:	48 8b ca             	mov    rcx,rdx
   1805915d6:	8b d0                	mov    edx,eax
   1805915d8:	4d 8b c1             	mov    r8,r9
   1805915db:	e9 b0 08 b4 ff       	jmp    0x1800d1e90

Exports::Context::Update

/home/phulin/c6-assets/DLC/Expansion2/Binaries/Win64/GameCore_XP2_FinalRelease.dll:     file format pei-x86-64


Disassembly of section .text:

0000000180591900 <.text+0x590900>:
   180591900:	48 83 ec 28          	sub    rsp,0x28
   180591904:	84 d2                	test   dl,dl
   180591906:	75 05                	jne    0x18059190d
   180591908:	e8 73 0e ab ff       	call   0x180042780
   18059190d:	33 c0                	xor    eax,eax
   18059190f:	48 83 c4 28          	add    rsp,0x28
   180591913:	c3                   	ret

Unit::Operations::Manager::Add

/home/phulin/c6-assets/DLC/Expansion2/Binaries/Win64/GameCore_XP2_FinalRelease.dll:     file format pei-x86-64


Disassembly of section .text:

000000018003f9c0 <.text+0x3e9c0>:
   18003f9c0:	48 8b c4             	mov    rax,rsp
   18003f9c3:	4c 89 40 18          	mov    QWORD PTR [rax+0x18],r8
   18003f9c7:	48 89 50 10          	mov    QWORD PTR [rax+0x10],rdx
   18003f9cb:	48 89 48 08          	mov    QWORD PTR [rax+0x8],rcx
   18003f9cf:	55                   	push   rbp
   18003f9d0:	53                   	push   rbx
   18003f9d1:	57                   	push   rdi
   18003f9d2:	48 8d 68 a1          	lea    rbp,[rax-0x5f]
   18003f9d6:	48 81 ec e0 00 00 00 	sub    rsp,0xe0
   18003f9dd:	48 89 70 e0          	mov    QWORD PTR [rax-0x20],rsi
   18003f9e1:	83 cf ff             	or     edi,0xffffffff
   18003f9e4:	4c 89 60 d8          	mov    QWORD PTR [rax-0x28],r12
   18003f9e8:	0f 57 c0             	xorps  xmm0,xmm0
   18003f9eb:	45 33 e4             	xor    r12d,r12d
   18003f9ee:	4c 89 70 c8          	mov    QWORD PTR [rax-0x38],r14
   18003f9f2:	4c 89 78 c0          	mov    QWORD PTR [rax-0x40],r15
   18003f9f6:	49 8b f0             	mov    rsi,r8
   18003f9f9:	89 7d df             	mov    DWORD PTR [rbp-0x21],edi
   18003f9fc:	4c 8b f2             	mov    r14,rdx
   18003f9ff:	89 7d e3             	mov    DWORD PTR [rbp-0x1d],edi
   18003fa02:	48 8b d9             	mov    rbx,rcx
   18003fa05:	44 89 65 f7          	mov    DWORD PTR [rbp-0x9],r12d
   18003fa09:	44 89 65 ff          	mov    DWORD PTR [rbp-0x1],r12d
   18003fa0d:	48 c7 45 03 ff ff ff 	mov    QWORD PTR [rbp+0x3],0xffffffffffffffff
   18003fa14:	ff 
   18003fa15:	f3 0f 7f 45 e7       	movdqu XMMWORD PTR [rbp-0x19],xmm0
   18003fa1a:	45 85 c9             	test   r9d,r9d
   18003fa1d:	75 06                	jne    0x18003fa25
   18003fa1f:	45 38 60 20          	cmp    BYTE PTR [r8+0x20],r12b
   18003fa23:	eb 04                	jmp    0x18003fa29
   18003fa25:	41 83 f9 01          	cmp    r9d,0x1
   18003fa29:	74 06                	je     0x18003fa31
   18003fa2b:	44 88 65 7f          	mov    BYTE PTR [rbp+0x7f],r12b
   18003fa2f:	eb 0a                	jmp    0x18003fa3b
   18003fa31:	48 8b 01             	mov    rax,QWORD PTR [rcx]
   18003fa34:	c6 45 7f 01          	mov    BYTE PTR [rbp+0x7f],0x1
   18003fa38:	ff 50 68             	call   QWORD PTR [rax+0x68]
   18003fa3b:	44 38 66 21          	cmp    BYTE PTR [rsi+0x21],r12b
   18003fa3f:	74 13                	je     0x18003fa54
   18003fa41:	48 8b 03             	mov    rax,QWORD PTR [rbx]
   18003fa44:	41 83 c8 ff          	or     r8d,0xffffffff
   18003fa48:	49 8b d6             	mov    rdx,r14
   18003fa4b:	48 8b cb             	mov    rcx,rbx
   18003fa4e:	ff 90 b8 00 00 00    	call   QWORD PTR [rax+0xb8]
   18003fa54:	48 8b 03             	mov    rax,QWORD PTR [rbx]
   18003fa57:	48 8b cb             	mov    rcx,rbx
   18003fa5a:	8b 16                	mov    edx,DWORD PTR [rsi]
   18003fa5c:	ff 90 80 00 00 00    	call   QWORD PTR [rax+0x80]
   18003fa62:	48 89 45 a7          	mov    QWORD PTR [rbp-0x59],rax
   18003fa66:	4c 8b f8             	mov    r15,rax
   18003fa69:	48 85 c0             	test   rax,rax
   18003fa6c:	0f 84 eb 01 00 00    	je     0x18003fc5d
   18003fa72:	48 8b 0d 17 b0 b4 00 	mov    rcx,QWORD PTR [rip+0xb4b017]        # 0x180b8aa90
   18003fa79:	ba ea a3 cf 7d       	mov    edx,0x7dcfa3ea
   18003fa7e:	4c 89 ac 24 c8 00 00 	mov    QWORD PTR [rsp+0xc8],r13
   18003fa85:	00 
   18003fa86:	4c 8b 01             	mov    r8,QWORD PTR [rcx]
   18003fa89:	41 ff 50 50          	call   QWORD PTR [r8+0x50]
   18003fa8d:	48 89 45 0f          	mov    QWORD PTR [rbp+0xf],rax
   18003fa91:	4c 8b e8             	mov    r13,rax
   18003fa94:	48 85 c0             	test   rax,rax
   18003fa97:	0f 84 91 00 00 00    	je     0x18003fb2e
   18003fa9d:	48 8b 10             	mov    rdx,QWORD PTR [rax]
   18003faa0:	48 8b c8             	mov    rcx,rax
   18003faa3:	ff 52 38             	call   QWORD PTR [rdx+0x38]
   18003faa6:	84 c0                	test   al,al
   18003faa8:	0f 84 80 00 00 00    	je     0x18003fb2e
   18003faae:	48 8b 15 b3 af b4 00 	mov    rdx,QWORD PTR [rip+0xb4afb3]        # 0x180b8aa68
   18003fab5:	48 8b cb             	mov    rcx,rbx
   18003fab8:	48 8b 45 77          	mov    rax,QWORD PTR [rbp+0x77]
   18003fabc:	45 8b be b0 00 00 00 	mov    r15d,DWORD PTR [r14+0xb0]
   18003fac3:	41 8b b6 c8 00 00 00 	mov    esi,DWORD PTR [r14+0xc8]
   18003faca:	45 8b a6 28 01 00 00 	mov    r12d,DWORD PTR [r14+0x128]
   18003fad1:	8b 38                	mov    edi,DWORD PTR [rax]
   18003fad3:	44 8b b2 c0 00 00 00 	mov    r14d,DWORD PTR [rdx+0xc0]
   18003fada:	8b d7                	mov    edx,edi
   18003fadc:	48 8b 03             	mov    rax,QWORD PTR [rbx]
   18003fadf:	4d 8b 6d 00          	mov    r13,QWORD PTR [r13+0x0]
   18003fae3:	ff 90 a8 00 00 00    	call   QWORD PTR [rax+0xa8]
   18003fae9:	8b ce                	mov    ecx,esi
   18003faeb:	48 8b d8             	mov    rbx,rax
   18003faee:	e8 bd 24 17 00       	call   0x1801b1fb0
   18003faf3:	48 8b 4d 0f          	mov    rcx,QWORD PTR [rbp+0xf]
   18003faf7:	48 8d 15 7a fc 97 00 	lea    rdx,[rip+0x97fc7a]        # 0x1809bf778
   18003fafe:	89 7c 24 38          	mov    DWORD PTR [rsp+0x38],edi
   18003fb02:	45 8b cc             	mov    r9d,r12d
   18003fb05:	48 89 5c 24 30       	mov    QWORD PTR [rsp+0x30],rbx
   18003fb0a:	45 8b c6             	mov    r8d,r14d
   18003fb0d:	44 89 7c 24 28       	mov    DWORD PTR [rsp+0x28],r15d
   18003fb12:	48 89 44 24 20       	mov    QWORD PTR [rsp+0x20],rax
   18003fb17:	41 ff 55 10          	call   QWORD PTR [r13+0x10]
   18003fb1b:	48 8b 5d 67          	mov    rbx,QWORD PTR [rbp+0x67]
   18003fb1f:	83 cf ff             	or     edi,0xffffffff
   18003fb22:	48 8b 75 77          	mov    rsi,QWORD PTR [rbp+0x77]
   18003fb26:	4c 8b 75 6f          	mov    r14,QWORD PTR [rbp+0x6f]
   18003fb2a:	4c 8b 7d a7          	mov    r15,QWORD PTR [rbp-0x59]
   18003fb2e:	8b 06                	mov    eax,DWORD PTR [rsi]
   18003fb30:	0f 57 c0             	xorps  xmm0,xmm0
   18003fb33:	4c 8b 46 10          	mov    r8,QWORD PTR [rsi+0x10]
   18003fb37:	48 8b 56 08          	mov    rdx,QWORD PTR [rsi+0x8]
   18003fb3b:	4c 8b ac 24 c8 00 00 	mov    r13,QWORD PTR [rsp+0xc8]
   18003fb42:	00 
   18003fb43:	89 45 b7             	mov    DWORD PTR [rbp-0x49],eax
   18003fb46:	8b 46 04             	mov    eax,DWORD PTR [rsi+0x4]
   18003fb49:	89 45 bb             	mov    DWORD PTR [rbp-0x45],eax
   18003fb4c:	f3 0f 7f 45 bf       	movdqu XMMWORD PTR [rbp-0x41],xmm0
   18003fb51:	4d 85 c0             	test   r8,r8
   18003fb54:	74 05                	je     0x18003fb5b
   18003fb56:	f0 41 ff 40 08       	lock inc DWORD PTR [r8+0x8]
   18003fb5b:	48 8d 4d bf          	lea    rcx,[rbp-0x41]
   18003fb5f:	e8 3c 06 0b 00       	call   0x1800f01a0
   18003fb64:	8b 46 18             	mov    eax,DWORD PTR [rsi+0x18]
   18003fb67:	4c 8d 45 b7          	lea    r8,[rbp-0x49]
   18003fb6b:	89 45 cf             	mov    DWORD PTR [rbp-0x31],eax
   18003fb6e:	49 8b d6             	mov    rdx,r14
   18003fb71:	0f b6 46 20          	movzx  eax,BYTE PTR [rsi+0x20]
   18003fb75:	49 8b cf             	mov    rcx,r15
   18003fb78:	88 45 d7             	mov    BYTE PTR [rbp-0x29],al
   18003fb7b:	0f b6 46 21          	movzx  eax,BYTE PTR [rsi+0x21]
   18003fb7f:	88 45 d8             	mov    BYTE PTR [rbp-0x28],al
   18003fb82:	49 8b 07             	mov    rax,QWORD PTR [r15]
   18003fb85:	ff 50 58             	call   QWORD PTR [rax+0x58]
   18003fb88:	84 c0                	test   al,al
   18003fb8a:	0f 84 9b 00 00 00    	je     0x18003fc2b
   18003fb90:	8b 45 b7             	mov    eax,DWORD PTR [rbp-0x49]
   18003fb93:	48 8d 55 bf          	lea    rdx,[rbp-0x41]
   18003fb97:	89 45 df             	mov    DWORD PTR [rbp-0x21],eax
   18003fb9a:	48 8d 4d e7          	lea    rcx,[rbp-0x19]
   18003fb9e:	8b 45 bb             	mov    eax,DWORD PTR [rbp-0x45]
   18003fba1:	89 45 e3             	mov    DWORD PTR [rbp-0x1d],eax
   18003fba4:	e8 37 92 69 00       	call   0x1806d8de0
   18003fba9:	8b 45 cf             	mov    eax,DWORD PTR [rbp-0x31]
   18003fbac:	48 8d 55 df          	lea    rdx,[rbp-0x21]
   18003fbb0:	89 45 f7             	mov    DWORD PTR [rbp-0x9],eax
   18003fbb3:	4d 8b c6             	mov    r8,r14
   18003fbb6:	48 8b 03             	mov    rax,QWORD PTR [rbx]
   18003fbb9:	48 8b cb             	mov    rcx,rbx
   18003fbbc:	ff 90 28 01 00 00    	call   QWORD PTR [rax+0x128]
   18003fbc2:	48 8b 03             	mov    rax,QWORD PTR [rbx]
   18003fbc5:	4c 8d 45 df          	lea    r8,[rbp-0x21]
   18003fbc9:	44 0f b6 4d 7f       	movzx  r9d,BYTE PTR [rbp+0x7f]
   18003fbce:	49 8b d6             	mov    rdx,r14
   18003fbd1:	48 8b cb             	mov    rcx,rbx
   18003fbd4:	ff 90 60 01 00 00    	call   QWORD PTR [rax+0x160]
   18003fbda:	80 7d d8 00          	cmp    BYTE PTR [rbp-0x28],0x0
   18003fbde:	74 4b                	je     0x18003fc2b
   18003fbe0:	48 8b 0d 81 ae b4 00 	mov    rcx,QWORD PTR [rip+0xb4ae81]        # 0x180b8aa68
   18003fbe7:	41 8b 9e 28 01 00 00 	mov    ebx,DWORD PTR [r14+0x128]
   18003fbee:	e8 8d d5 5f 00       	call   0x18063d180
   18003fbf3:	3b d8                	cmp    ebx,eax
   18003fbf5:	75 34                	jne    0x18003fc2b
   18003fbf7:	41 0f b7 86 28 01 00 	movzx  eax,WORD PTR [r14+0x128]
   18003fbfe:	00 
   18003fbff:	48 8d 55 77          	lea    rdx,[rbp+0x77]
   18003fc03:	66 89 45 77          	mov    WORD PTR [rbp+0x77],ax
   18003fc07:	b8 01 00 00 00       	mov    eax,0x1
   18003fc0c:	66 89 45 79          	mov    WORD PTR [rbp+0x79],ax
   18003fc10:	41 8b 86 b0 00 00 00 	mov    eax,DWORD PTR [r14+0xb0]
   18003fc17:	48 c7 45 a7 e9 64 42 	mov    QWORD PTR [rbp-0x59],0x7b4264e9
   18003fc1e:	7b 
   18003fc1f:	48 8b 4d a7          	mov    rcx,QWORD PTR [rbp-0x59]
   18003fc23:	89 45 7b             	mov    DWORD PTR [rbp+0x7b],eax
   18003fc26:	e8 d5 b3 5b 00       	call   0x1805fb000
   18003fc2b:	48 8b 5d c7          	mov    rbx,QWORD PTR [rbp-0x39]
   18003fc2f:	48 85 db             	test   rbx,rbx
   18003fc32:	74 29                	je     0x18003fc5d
   18003fc34:	8b c7                	mov    eax,edi
   18003fc36:	f0 0f c1 43 08       	lock xadd DWORD PTR [rbx+0x8],eax
   18003fc3b:	83 f8 01             	cmp    eax,0x1
   18003fc3e:	75 1d                	jne    0x18003fc5d
   18003fc40:	48 8b 03             	mov    rax,QWORD PTR [rbx]
   18003fc43:	48 8b cb             	mov    rcx,rbx
   18003fc46:	ff 10                	call   QWORD PTR [rax]
   18003fc48:	8b c7                	mov    eax,edi
   18003fc4a:	f0 0f c1 43 0c       	lock xadd DWORD PTR [rbx+0xc],eax
   18003fc4f:	83 f8 01             	cmp    eax,0x1
   18003fc52:	75 09                	jne    0x18003fc5d
   18003fc54:	48 8b 03             	mov    rax,QWORD PTR [rbx]
   18003fc57:	48 8b cb             	mov    rcx,rbx
   18003fc5a:	ff 50 08             	call   QWORD PTR [rax+0x8]
   18003fc5d:	48 8b 5d ef          	mov    rbx,QWORD PTR [rbp-0x11]
   18003fc61:	4c 8b bc 24 b8 00 00 	mov    r15,QWORD PTR [rsp+0xb8]
   18003fc68:	00 
   18003fc69:	4c 8b b4 24 c0 00 00 	mov    r14,QWORD PTR [rsp+0xc0]
   18003fc70:	00 
   18003fc71:	4c 8b a4 24 d0 00 00 	mov    r12,QWORD PTR [rsp+0xd0]
   18003fc78:	00 
   18003fc79:	48 8b b4 24 d8 00 00 	mov    rsi,QWORD PTR [rsp+0xd8]
   18003fc80:	00 
   18003fc81:	48 85 db             	test   rbx,rbx
   18003fc84:	74 27                	je     0x18003fcad
   18003fc86:	8b c7                	mov    eax,edi
   18003fc88:	f0 0f c1 43 08       	lock xadd DWORD PTR [rbx+0x8],eax
   18003fc8d:	83 f8 01             	cmp    eax,0x1
   18003fc90:	75 1b                	jne    0x18003fcad
   18003fc92:	48 8b 03             	mov    rax,QWORD PTR [rbx]
   18003fc95:	48 8b cb             	mov    rcx,rbx
   18003fc98:	ff 10                	call   QWORD PTR [rax]
   18003fc9a:	f0 0f c1 7b 0c       	lock xadd DWORD PTR [rbx+0xc],edi
   18003fc9f:	83 ff 01             	cmp    edi,0x1
   18003fca2:	75 09                	jne    0x18003fcad
   18003fca4:	48 8b 03             	mov    rax,QWORD PTR [rbx]
   18003fca7:	48 8b cb             	mov    rcx,rbx
   18003fcaa:	ff 50 08             	call   QWORD PTR [rax+0x8]
   18003fcad:	48 81 c4 e0 00 00 00 	add    rsp,0xe0
   18003fcb4:	5f                   	pop    rdi
   18003fcb5:	5b                   	pop    rbx
   18003fcb6:	5d                   	pop    rbp
   18003fcb7:	c3                   	ret

Cache::Unit::Manager::RequestOperation

/home/phulin/c6-assets/DLC/Expansion2/Binaries/Win64/GameCore_XP2_FinalRelease.dll:     file format pei-x86-64


Disassembly of section .text:

00000001800d1e90 <.text+0xd0e90>:
   1800d1e90:	48 89 5c 24 10       	mov    QWORD PTR [rsp+0x10],rbx
   1800d1e95:	48 89 6c 24 18       	mov    QWORD PTR [rsp+0x18],rbp
   1800d1e9a:	48 89 74 24 20       	mov    QWORD PTR [rsp+0x20],rsi
   1800d1e9f:	41 56                	push   r14
   1800d1ea1:	48 81 ec a0 00 00 00 	sub    rsp,0xa0
   1800d1ea8:	0f b7 19             	movzx  ebx,WORD PTR [rcx]
   1800d1eab:	49 8b e8             	mov    rbp,r8
   1800d1eae:	44 8b f2             	mov    r14d,edx
   1800d1eb1:	48 8b f1             	mov    rsi,rcx
   1800d1eb4:	66 83 fb 40          	cmp    bx,0x40
   1800d1eb8:	0f 83 d4 00 00 00    	jae    0x1800d1f92
   1800d1ebe:	e8 0d d6 fe ff       	call   0x1800bf4d0
   1800d1ec3:	4c 0f bf cb          	movsx  r9,bx
   1800d1ec7:	48 8b 40 20          	mov    rax,QWORD PTR [rax+0x20]
   1800d1ecb:	4e 8b 0c c8          	mov    r9,QWORD PTR [rax+r9*8]
   1800d1ecf:	41 80 79 20 00       	cmp    BYTE PTR [r9+0x20],0x0
   1800d1ed4:	0f 84 b8 00 00 00    	je     0x1800d1f92
   1800d1eda:	0f 57 c0             	xorps  xmm0,xmm0
   1800d1edd:	48 89 bc 24 b0 00 00 	mov    QWORD PTR [rsp+0xb0],rdi
   1800d1ee4:	00 
   1800d1ee5:	83 cf ff             	or     edi,0xffffffff
   1800d1ee8:	c7 44 24 38 00 00 00 	mov    DWORD PTR [rsp+0x38],0x0
   1800d1eef:	00 
   1800d1ef0:	89 7c 24 20          	mov    DWORD PTR [rsp+0x20],edi
   1800d1ef4:	89 7c 24 24          	mov    DWORD PTR [rsp+0x24],edi
   1800d1ef8:	f3 0f 7f 44 24 28    	movdqu XMMWORD PTR [rsp+0x28],xmm0
   1800d1efe:	66 c7 44 24 40 00 00 	mov    WORD PTR [rsp+0x40],0x0
   1800d1f05:	e8 e6 e9 f6 ff       	call   0x1800408f0
   1800d1f0a:	4c 8d 4c 24 20       	lea    r9,[rsp+0x20]
   1800d1f0f:	4c 8b c5             	mov    r8,rbp
   1800d1f12:	41 8b d6             	mov    edx,r14d
   1800d1f15:	48 8b c8             	mov    rcx,rax
   1800d1f18:	4c 8b 10             	mov    r10,QWORD PTR [rax]
   1800d1f1b:	41 ff 92 f8 00 00 00 	call   QWORD PTR [r10+0xf8]
   1800d1f22:	84 c0                	test   al,al
   1800d1f24:	74 33                	je     0x1800d1f59
   1800d1f26:	48 8d 4c 24 50       	lea    rcx,[rsp+0x50]
   1800d1f2b:	e8 10 87 f6 ff       	call   0x18003a640
   1800d1f30:	45 33 c9             	xor    r9d,r9d
   1800d1f33:	4c 8d 44 24 20       	lea    r8,[rsp+0x20]
   1800d1f38:	48 8b d6             	mov    rdx,rsi
   1800d1f3b:	48 8d 4c 24 50       	lea    rcx,[rsp+0x50]
   1800d1f40:	e8 1b 8c f6 ff       	call   0x18003ab60
   1800d1f45:	48 8d 4c 24 50       	lea    rcx,[rsp+0x50]
   1800d1f4a:	e8 61 47 f6 ff       	call   0x1800366b0
   1800d1f4f:	48 8d 4c 24 50       	lea    rcx,[rsp+0x50]
   1800d1f54:	e8 a7 83 f6 ff       	call   0x18003a300
   1800d1f59:	48 8b 5c 24 30       	mov    rbx,QWORD PTR [rsp+0x30]
   1800d1f5e:	48 85 db             	test   rbx,rbx
   1800d1f61:	74 27                	je     0x1800d1f8a
   1800d1f63:	8b c7                	mov    eax,edi
   1800d1f65:	f0 0f c1 43 08       	lock xadd DWORD PTR [rbx+0x8],eax
   1800d1f6a:	83 f8 01             	cmp    eax,0x1
   1800d1f6d:	75 1b                	jne    0x1800d1f8a
   1800d1f6f:	48 8b 03             	mov    rax,QWORD PTR [rbx]
   1800d1f72:	48 8b cb             	mov    rcx,rbx
   1800d1f75:	ff 10                	call   QWORD PTR [rax]
   1800d1f77:	f0 0f c1 7b 0c       	lock xadd DWORD PTR [rbx+0xc],edi
   1800d1f7c:	83 ff 01             	cmp    edi,0x1
   1800d1f7f:	75 09                	jne    0x1800d1f8a
   1800d1f81:	48 8b 03             	mov    rax,QWORD PTR [rbx]
   1800d1f84:	48 8b cb             	mov    rcx,rbx
   1800d1f87:	ff 50 08             	call   QWORD PTR [rax+0x8]
   1800d1f8a:	48 8b bc 24 b0 00 00 	mov    rdi,QWORD PTR [rsp+0xb0]
   1800d1f91:	00 
   1800d1f92:	4c 8d 9c 24 a0 00 00 	lea    r11,[rsp+0xa0]
   1800d1f99:	00 
   1800d1f9a:	49 8b 5b 18          	mov    rbx,QWORD PTR [r11+0x18]
   1800d1f9e:	49 8b 6b 20          	mov    rbp,QWORD PTR [r11+0x20]
   1800d1fa2:	49 8b 73 28          	mov    rsi,QWORD PTR [r11+0x28]
   1800d1fa6:	49 8b e3             	mov    rsp,r11
   1800d1fa9:	41 5e                	pop    r14
   1800d1fab:	c3                   	ret

Player::TurnManager::DoTurn

/home/phulin/c6-assets/DLC/Expansion2/Binaries/Win64/GameCore_XP2_FinalRelease.dll:     file format pei-x86-64


Disassembly of section .text:

0000000180344a00 <.text+0x343a00>:
   180344a00:	40 53                	rex push rbx
   180344a02:	48 83 ec 20          	sub    rsp,0x20
   180344a06:	48 8b d9             	mov    rbx,rcx
   180344a09:	48 8b 09             	mov    rcx,QWORD PTR [rcx]
   180344a0c:	e8 0f 4e fb ff       	call   0x1802f9820
   180344a11:	84 c0                	test   al,al
   180344a13:	75 14                	jne    0x180344a29
   180344a15:	48 8b 0b             	mov    rcx,QWORD PTR [rbx]
   180344a18:	48 8b 89 b0 06 00 00 	mov    rcx,QWORD PTR [rcx+0x6b0]
   180344a1f:	48 83 c4 20          	add    rsp,0x20
   180344a23:	5b                   	pop    rbx
   180344a24:	e9 27 c6 0b 00       	jmp    0x180401050
   180344a29:	48 83 c4 20          	add    rsp,0x20
   180344a2d:	5b                   	pop    rbx
   180344a2e:	c3                   	ret

Game::Instance::Update

/home/phulin/c6-assets/DLC/Expansion2/Binaries/Win64/GameCore_XP2_FinalRelease.dll:     file format pei-x86-64


Disassembly of section .text:

000000018063e2e0 <.text+0x63d2e0>:
   18063e2e0:	48 83 ec 28          	sub    rsp,0x28
   18063e2e4:	e8 d7 cc a0 ff       	call   0x18004afc0
   18063e2e9:	e8 72 8b b4 ff       	call   0x180186e60
   18063e2ee:	48 83 c4 28          	add    rsp,0x28
   18063e2f2:	e9 e9 d8 a3 ff       	jmp    0x18007bbe0
