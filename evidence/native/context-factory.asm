
/home/phulin/c6-assets/DLC/Expansion2/Binaries/Win64/GameCore_XP2_FinalRelease.dll:     file format pei-x86-64


Disassembly of section .text:

000000018058fc60 <.text+0x58ec60>:
   18058fc60:	48 83 ec 28          	sub    $0x28,%rsp
   18058fc64:	48 8b 0d 35 7a 5d 00 	mov    0x5d7a35(%rip),%rcx        # 0x180b676a0
   18058fc6b:	48 83 f9 ff          	cmp    $0xffffffffffffffff,%rcx
   18058fc6f:	75 39                	jne    0x18058fcaa
   18058fc71:	45 33 c0             	xor    %r8d,%r8d
   18058fc74:	33 d2                	xor    %edx,%edx
   18058fc76:	33 c9                	xor    %ecx,%ecx
   18058fc78:	ff 15 c2 75 42 00    	call   *0x4275c2(%rip)        # 0x1809b7240
   18058fc7e:	41 b9 04 00 00 00    	mov    $0x4,%r9d
   18058fc84:	c7 44 24 30 02 00 00 	movl   $0x2,0x30(%rsp)
   18058fc8b:	00 
   18058fc8c:	48 8b c8             	mov    %rax,%rcx
   18058fc8f:	48 89 05 0a 7a 5d 00 	mov    %rax,0x5d7a0a(%rip)        # 0x180b676a0
   18058fc96:	4c 8d 44 24 30       	lea    0x30(%rsp),%r8
   18058fc9b:	33 d2                	xor    %edx,%edx
   18058fc9d:	ff 15 bd 75 42 00    	call   *0x4275bd(%rip)        # 0x1809b7260
   18058fca3:	48 8b 0d f6 79 5d 00 	mov    0x5d79f6(%rip),%rcx        # 0x180b676a0
   18058fcaa:	33 d2                	xor    %edx,%edx
   18058fcac:	44 8d 42 10          	lea    0x10(%rdx),%r8d
   18058fcb0:	ff 15 9a 75 42 00    	call   *0x42759a(%rip)        # 0x1809b7250
   18058fcb6:	48 85 c0             	test   %rax,%rax
   18058fcb9:	74 08                	je     0x18058fcc3
   18058fcbb:	48 8b c8             	mov    %rax,%rcx
   18058fcbe:	e8 7d fe ff ff       	call   0x18058fb40
   18058fcc3:	48 8b 15 fe f3 5f 00 	mov    0x5ff3fe(%rip),%rdx        # 0x180b8f0c8
   18058fcca:	48 3b 15 ff f3 5f 00 	cmp    0x5ff3ff(%rip),%rdx        # 0x180b8f0d0
   18058fcd1:	48 89 44 24 30       	mov    %rax,0x30(%rsp)
   18058fcd6:	73 1b                	jae    0x18058fcf3
   18058fcd8:	48 8b ca             	mov    %rdx,%rcx
   18058fcdb:	48 83 c2 08          	add    $0x8,%rdx
   18058fcdf:	48 89 15 e2 f3 5f 00 	mov    %rdx,0x5ff3e2(%rip)        # 0x180b8f0c8
   18058fce6:	48 85 c9             	test   %rcx,%rcx
   18058fce9:	74 1e                	je     0x18058fd09
   18058fceb:	48 89 01             	mov    %rax,(%rcx)
   18058fcee:	48 83 c4 28          	add    $0x28,%rsp
   18058fcf2:	c3                   	ret
   18058fcf3:	4c 8d 44 24 30       	lea    0x30(%rsp),%r8
   18058fcf8:	48 8d 0d c1 f3 5f 00 	lea    0x5ff3c1(%rip),%rcx        # 0x180b8f0c0
   18058fcff:	e8 6c 3b a8 ff       	call   0x180013870
   18058fd04:	48 8b 44 24 30       	mov    0x30(%rsp),%rax
   18058fd09:	48 83 c4 28          	add    $0x28,%rsp
   18058fd0d:	c3                   	ret
   18058fd0e:	cc                   	int3
   18058fd0f:	cc                   	int3
   18058fd10:	48 85 c9             	test   %rcx,%rcx
   18058fd13:	0f 84 e4 00 00 00    	je     0x18058fdfd
   18058fd19:	53                   	push   %rbx
   18058fd1a:	48 83 ec 20          	sub    $0x20,%rsp
   18058fd1e:	48 8b 05 a3 f3 5f 00 	mov    0x5ff3a3(%rip),%rax        # 0x180b8f0c8
   18058fd25:	48 8b d9             	mov    %rcx,%rbx
   18058fd28:	48 8b 0d 91 f3 5f 00 	mov    0x5ff391(%rip),%rcx        # 0x180b8f0c0
   18058fd2f:	48 3b c8             	cmp    %rax,%rcx
   18058fd32:	74 37                	je     0x18058fd6b
   18058fd34:	48 39 19             	cmp    %rbx,(%rcx)
   18058fd37:	74 0b                	je     0x18058fd44
   18058fd39:	48 83 c1 08          	add    $0x8,%rcx
   18058fd3d:	48 3b c8             	cmp    %rax,%rcx
   18058fd40:	75 f2                	jne    0x18058fd34
   18058fd42:	eb 27                	jmp    0x18058fd6b
   18058fd44:	48 8d 51 08          	lea    0x8(%rcx),%rdx
   18058fd48:	48 3b d0             	cmp    %rax,%rdx
   18058fd4b:	73 13                	jae    0x18058fd60
   18058fd4d:	48 2b c2             	sub    %rdx,%rax
   18058fd50:	4c 8b c0             	mov    %rax,%r8
   18058fd53:	ff 15 a7 78 42 00    	call   *0x4278a7(%rip)        # 0x1809b7600
   18058fd59:	48 8b 05 68 f3 5f 00 	mov    0x5ff368(%rip),%rax        # 0x180b8f0c8
   18058fd60:	48 83 e8 08          	sub    $0x8,%rax
   18058fd64:	48 89 05 5d f3 5f 00 	mov    %rax,0x5ff35d(%rip)        # 0x180b8f0c8
   18058fd6b:	48 8b 4b 08          	mov    0x8(%rbx),%rcx
   18058fd6f:	48 8d 05 72 04 48 00 	lea    0x480472(%rip),%rax        # 0x180a101e8
   18058fd76:	48 89 03             	mov    %rax,(%rbx)
   18058fd79:	48 85 c9             	test   %rcx,%rcx
   18058fd7c:	74 13                	je     0x18058fd91
   18058fd7e:	e8 0d 65 ab ff       	call   0x180046290
   18058fd83:	84 c0                	test   %al,%al
   18058fd85:	74 0a                	je     0x18058fd91
   18058fd87:	48 8b 4b 08          	mov    0x8(%rbx),%rcx
   18058fd8b:	48 8b 01             	mov    (%rcx),%rax
   18058fd8e:	ff 50 10             	call   *0x10(%rax)
   18058fd91:	48 8b 4b 08          	mov    0x8(%rbx),%rcx
   18058fd95:	48 85 c9             	test   %rcx,%rcx
   18058fd98:	74 0a                	je     0x18058fda4
   18058fd9a:	48 8b 01             	mov    (%rcx),%rax
   18058fd9d:	ba 01 00 00 00       	mov    $0x1,%edx
   18058fda2:	ff 10                	call   *(%rax)
   18058fda4:	48 8b 0d f5 78 5d 00 	mov    0x5d78f5(%rip),%rcx        # 0x180b676a0
   18058fdab:	48 c7 43 08 00 00 00 	movq   $0x0,0x8(%rbx)
   18058fdb2:	00 
   18058fdb3:	48 83 f9 ff          	cmp    $0xffffffffffffffff,%rcx
   18058fdb7:	74 12                	je     0x18058fdcb
   18058fdb9:	4c 8b c3             	mov    %rbx,%r8
   18058fdbc:	33 d2                	xor    %edx,%edx
   18058fdbe:	ff 15 94 74 42 00    	call   *0x427494(%rip)        # 0x1809b7258
   18058fdc4:	48 8b 0d d5 78 5d 00 	mov    0x5d78d5(%rip),%rcx        # 0x180b676a0
   18058fdcb:	48 8b 05 f6 f2 5f 00 	mov    0x5ff2f6(%rip),%rax        # 0x180b8f0c8
   18058fdd2:	48 2b 05 e7 f2 5f 00 	sub    0x5ff2e7(%rip),%rax        # 0x180b8f0c0
   18058fdd9:	48 c1 f8 03          	sar    $0x3,%rax
   18058fddd:	85 c0                	test   %eax,%eax
   18058fddf:	75 17                	jne    0x18058fdf8
   18058fde1:	48 83 f9 ff          	cmp    $0xffffffffffffffff,%rcx
   18058fde5:	74 11                	je     0x18058fdf8
   18058fde7:	ff 15 5b 74 42 00    	call   *0x42745b(%rip)        # 0x1809b7248
   18058fded:	48 c7 05 a8 78 5d 00 	movq   $0xffffffffffffffff,0x5d78a8(%rip)        # 0x180b676a0
   18058fdf4:	ff ff ff ff 
   18058fdf8:	48 83 c4 20          	add    $0x20,%rsp
   18058fdfc:	5b                   	pop    %rbx
   18058fdfd:	c3                   	ret
   18058fdfe:	cc                   	int3
   18058fdff:	cc                   	int3
   18058fe00:	48 83 ec 28          	sub    $0x28,%rsp
   18058fe04:	48 8b 15 bd f2 5f 00 	mov    0x5ff2bd(%rip),%rdx        # 0x180b8f0c8
   18058fe0b:	48 8b 05 ae f2 5f 00 	mov    0x5ff2ae(%rip),%rax        # 0x180b8f0c0
   18058fe12:	48 8b ca             	mov    %rdx,%rcx
   18058fe15:	48 2b c8             	sub    %rax,%rcx
   18058fe18:	48 c1 f9 03          	sar    $0x3,%rcx
   18058fe1c:	85 c9                	test   %ecx,%ecx
   18058fe1e:	0f 84 21 01 00 00    	je     0x18058ff45
   18058fe24:	48 89 5c 24 30       	mov    %rbx,0x30(%rsp)
   18058fe29:	48 89 74 24 38       	mov    %rsi,0x38(%rsp)
   18058fe2e:	48 8d 35 b3 03 48 00 	lea    0x4803b3(%rip),%rsi        # 0x180a101e8
   18058fe35:	48 89 7c 24 20       	mov    %rdi,0x20(%rsp)
   18058fe3a:	33 ff                	xor    %edi,%edi
   18058fe3c:	0f 1f 40 00          	nopl   0x0(%rax)
   18058fe40:	48 8b 18             	mov    (%rax),%rbx
   18058fe43:	48 85 db             	test   %rbx,%rbx
   18058fe46:	0f 84 d8 00 00 00    	je     0x18058ff24
   18058fe4c:	48 3b c2             	cmp    %rdx,%rax
   18058fe4f:	74                   	.byte 0x74
