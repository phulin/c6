
/home/phulin/c6-assets/DLC/Expansion2/Binaries/Win64/GameCore_XP2_FinalRelease.dll:     file format pei-x86-64


Disassembly of section .text:

00000001805f9ee0 <.text+0x5f8ee0>:
   1805f9ee0:	48 8d 05 11 30 42 00 	lea    0x423011(%rip),%rax        # 0x180a1cef8
   1805f9ee7:	48 c7 41 10 01 00 00 	movq   $0x1,0x10(%rcx)
   1805f9eee:	00 
   1805f9eef:	48 89 01             	mov    %rax,(%rcx)
   1805f9ef2:	33 c0                	xor    %eax,%eax
   1805f9ef4:	48 89 41 08          	mov    %rax,0x8(%rcx)
   1805f9ef8:	88 41 18             	mov    %al,0x18(%rcx)
   1805f9efb:	48 8b c1             	mov    %rcx,%rax
   1805f9efe:	c3                   	ret
   1805f9eff:	cc                   	int3
   1805f9f00:	48 8d 05 f1 2f 42 00 	lea    0x422ff1(%rip),%rax        # 0x180a1cef8
   1805f9f07:	48 89 01             	mov    %rax,(%rcx)
   1805f9f0a:	c3                   	ret
   1805f9f0b:	cc                   	int3
   1805f9f0c:	cc                   	int3
   1805f9f0d:	cc                   	int3
   1805f9f0e:	cc                   	int3
   1805f9f0f:	cc                   	int3
   1805f9f10:	40 53                	rex push %rbx
   1805f9f12:	48 83 ec 20          	sub    $0x20,%rsp
   1805f9f16:	48 8b c2             	mov    %rdx,%rax
   1805f9f19:	48 8b d9             	mov    %rcx,%rbx
   1805f9f1c:	48 8b d1             	mov    %rcx,%rdx
   1805f9f1f:	48 8b c8             	mov    %rax,%rcx
   1805f9f22:	e8 69 02 00 00       	call   0x1805fa190
   1805f9f27:	48 8b c3             	mov    %rbx,%rax
   1805f9f2a:	48 83 c4 20          	add    $0x20,%rsp
   1805f9f2e:	5b                   	pop    %rbx
   1805f9f2f:	c3                   	ret
   1805f9f30:	48 89 5c 24 08       	mov    %rbx,0x8(%rsp)
   1805f9f35:	57                   	push   %rdi
   1805f9f36:	48 83 ec 50          	sub    $0x50,%rsp
   1805f9f3a:	48 8b fa             	mov    %rdx,%rdi
   1805f9f3d:	4c 8d 05 b9 67 59 00 	lea    0x5967b9(%rip),%r8        # 0x180b906fd
   1805f9f44:	ba 00 01 00 00       	mov    $0x100,%edx
   1805f9f49:	4c 89 44 24 40       	mov    %r8,0x40(%rsp)
   1805f9f4e:	48 8b d9             	mov    %rcx,%rbx
   1805f9f51:	48 8d 0d a4 67 59 00 	lea    0x5967a4(%rip),%rcx        # 0x180b906fc
   1805f9f58:	8b c1                	mov    %ecx,%eax
   1805f9f5a:	48 89 4c 24 30       	mov    %rcx,0x30(%rsp)
   1805f9f5f:	2b c1                	sub    %ecx,%eax
   1805f9f61:	48 89 4c 24 38       	mov    %rcx,0x38(%rsp)
   1805f9f66:	3b c2                	cmp    %edx,%eax
   1805f9f68:	0f 47 d0             	cmova  %eax,%edx
   1805f9f6b:	44 2b c1             	sub    %ecx,%r8d
   1805f9f6e:	41 3b d0             	cmp    %r8d,%edx
   1805f9f71:	72 0a                	jb     0x1805f9f7d
   1805f9f73:	48 8d 4c 24 30       	lea    0x30(%rsp),%rcx
   1805f9f78:	e8 43 ae a1 ff       	call   0x180014dc0
   1805f9f7d:	0f b6 47 18          	movzbl 0x18(%rdi),%eax
   1805f9f81:	48 8d 15 e8 2f 42 00 	lea    0x422fe8(%rip),%rdx        # 0x180a1cf70
   1805f9f88:	44 8b 4f 0c          	mov    0xc(%rdi),%r9d
   1805f9f8c:	48 8d 4c 24 30       	lea    0x30(%rsp),%rcx
   1805f9f91:	44 8b 47 08          	mov    0x8(%rdi),%r8d
   1805f9f95:	89 44 24 28          	mov    %eax,0x28(%rsp)
   1805f9f99:	8b 47 10             	mov    0x10(%rdi),%eax
   1805f9f9c:	89 44 24 20          	mov    %eax,0x20(%rsp)
   1805f9fa0:	e8 3b af a1 ff       	call   0x180014ee0
   1805f9fa5:	4c 8b 44 24 38       	mov    0x38(%rsp),%r8
   1805f9faa:	48 8b cb             	mov    %rbx,%rcx
   1805f9fad:	48 8b 54 24 30       	mov    0x30(%rsp),%rdx
   1805f9fb2:	e8 c9 aa a1 ff       	call   0x180014a80
   1805f9fb7:	48 8b 4c 24 40       	mov    0x40(%rsp),%rcx
   1805f9fbc:	48 8b 54 24 30       	mov    0x30(%rsp),%rdx
   1805f9fc1:	48 8b c1             	mov    %rcx,%rax
   1805f9fc4:	48 2b c2             	sub    %rdx,%rax
   1805f9fc7:	48 83 f8 01          	cmp    $0x1,%rax
   1805f9fcb:	7e 14                	jle    0x1805f9fe1
   1805f9fcd:	48 85 d2             	test   %rdx,%rdx
   1805f9fd0:	74 0f                	je     0x1805f9fe1
   1805f9fd2:	2b ca                	sub    %edx,%ecx
   1805f9fd4:	44 8b c1             	mov    %ecx,%r8d
   1805f9fd7:	48 8d 4c 24 48       	lea    0x48(%rsp),%rcx
   1805f9fdc:	e8 3f ce a3 ff       	call   0x180036e20
   1805f9fe1:	48 8b c3             	mov    %rbx,%rax
   1805f9fe4:	48 8b 5c 24 60       	mov    0x60(%rsp),%rbx
   1805f9fe9:	48 83 c4 50          	add    $0x50,%rsp
   1805f9fed:	5f                   	pop    %rdi
   1805f9fee:	c3                   	ret
   1805f9fef:	cc                   	int3
   1805f9ff0:	40 53                	rex push %rbx
   1805f9ff2:	48 83 ec 20          	sub    $0x20,%rsp
   1805f9ff6:	48 8b c2             	mov    %rdx,%rax
   1805f9ff9:	48 8b d9             	mov    %rcx,%rbx
   1805f9ffc:	48 8b d1             	mov    %rcx,%rdx
   1805f9fff:	48 8b c8             	mov    %rax,%rcx
   1805fa002:	e8 89 03 00 00       	call   0x1805fa390
   1805fa007:	48 8b c3             	mov    %rbx,%rax
   1805fa00a:	48 83 c4 20          	add    $0x20,%rsp
   1805fa00e:	5b                   	pop    %rbx
   1805fa00f:	c3                   	ret
   1805fa010:	40 53                	rex push %rbx
   1805fa012:	48 83 ec 20          	sub    $0x20,%rsp
   1805fa016:	48 8d 05 db 2e 42 00 	lea    0x422edb(%rip),%rax        # 0x180a1cef8
   1805fa01d:	48 8b d9             	mov    %rcx,%rbx
   1805fa020:	48 89 01             	mov    %rax,(%rcx)
   1805fa023:	f6 c2 01             	test   $0x1,%dl
   1805fa026:	74 0a                	je     0x1805fa032
   1805fa028:	ba 20 00 00 00       	mov    $0x20,%edx
   1805fa02d:	e8 76 5e 3b 00       	call   0x1809afea8
   1805fa032:	48 8b c3             	mov    %rbx,%rax
   1805fa035:	48 83 c4 20          	add    $0x20,%rsp
   1805fa039:	5b                   	pop    %rbx
   1805fa03a:	c3                   	ret
   1805fa03b:	cc                   	int3
   1805fa03c:	cc                   	int3
   1805fa03d:	cc                   	int3
   1805fa03e:	cc                   	int3
   1805fa03f:	cc                   	int3
   1805fa040:	48 89 5c 24 08       	mov    %rbx,0x8(%rsp)
   1805fa045:	48 89 6c 24 10       	mov    %rbp,0x10(%rsp)
   1805fa04a:	48 89 74 24 18       	mov    %rsi,0x18(%rsp)
   1805fa04f:	57                   	push   %rdi
   1805fa050:	41 56                	push   %r14
   1805fa052:	41 57                	push   %r15
   1805fa054:	48 83 ec 40          	sub    $0x40,%rsp
   1805fa058:	69 71 08 6d 4e c6 41 	imul   $0x41c64e6d,0x8(%rcx),%esi
   1805fa05f:	48 8b d9             	mov    %rcx,%rbx
   1805fa062:	ff 41 0c             	incl   0xc(%rcx)
   1805fa065:	49 8b e8             	mov    %r8,%rbp
   1805fa068:	48 8b 0d 21 0a 59 00 	mov    0x590a21(%rip),%rcx        # 0x180b8aa90
   1805fa06f:	44 0f b7 fa          	movzwl %dx,%r15d
   1805fa073:	ba ab 44 f1 fc       	mov    $0xfcf144ab,%edx
   1805fa078:	81 c6 39 30 00 00    	add    $0x3039,%esi
   1805fa07e:	48 8b 01             	mov    (%rcx),%rax
   1805fa081:	8b fe                	mov    %esi,%edi
   1805fa083:	c1 ef 10             	shr    $0x10,%edi
   1805fa086:	41 0f af ff          	imul   %r15d,%edi
   1805fa08a:	c1 ef 10             	shr    $0x10,%edi
   1805fa08d:	ff 50 50             	call   *0x50(%rax)
   1805fa090:	4c 8b f0             	mov    %rax,%r14
   1805fa093:	48 85 c0             	test   %rax,%rax
   1805fa096:	74 5f                	je     0x1805fa0f7
   1805fa098:	48 8b 00             	mov    (%rax),%rax
   1805fa09b:	49 8b ce             	mov    %r14,%rcx
   1805fa09e:	ff 50 38             	call   *0x38(%rax)
   1805fa0a1:	84 c0                	test   %al,%al
   1805fa0a3:	74 52                	je     0x1805fa0f7
   1805fa0a5:	80 7b 18 00          	cmpb   $0x0,0x18(%rbx)
   1805fa0a9:	74 4c                	je     0x1805fa0f7
   1805fa0ab:	4d 8b 16             	mov    (%r14),%r10
   1805fa0ae:	48 8d 05 97 2e 42 00 	lea    0x422e97(%rip),%rax        # 0x180a1cf4c
   1805fa0b5:	48 85 ed             	test   %rbp,%rbp
   1805fa0b8:	48 8d 0d 51 3d 3c 00 	lea    0x3c3d51(%rip),%rcx        # 0x1809bde10
   1805fa0bf:	45 8b cf             	mov    %r15d,%r9d
   1805fa0c2:	48 8d 15 8f 2e 42 00 	lea    0x422e8f(%rip),%rdx        # 0x180a1cf58
   1805fa0c9:	48 0f 45 cd          	cmovne %rbp,%rcx
   1805fa0cd:	48 89 4c 24 38       	mov    %rcx,0x38(%rsp)
   1805fa0d2:	8b 4b 08             	mov    0x8(%rbx),%ecx
   1805fa0d5:	48 89 44 24 30       	mov    %rax,0x30(%rsp)
   1805fa0da:	89 4c 24 28          	mov    %ecx,0x28(%rsp)
   1805fa0de:	48 8b 0d 83 09 59 00 	mov    0x590983(%rip),%rcx        # 0x180b8aa68
   1805fa0e5:	89 7c 24 20          	mov    %edi,0x20(%rsp)
   1805fa0e9:	44 8b 81 c0 00 00 00 	mov    0xc0(%rcx),%r8d
   1805fa0f0:	49 8b ce             	mov    %r14,%rcx
   1805fa0f3:	41 ff 52 10          	call   *0x10(%r10)
   1805fa0f7:	48 8b 6c 24 68       	mov    0x68(%rsp),%rbp
   1805fa0fc:	0f b7 c7             	movzwl %di,%eax
   1805fa0ff:	89 73 08             	mov    %esi,0x8(%rbx)
   1805fa102:	48 8b 5c 24 60       	mov    0x60(%rsp),%rbx
   1805fa107:	48 8b 74 24 70       	mov    0x70(%rsp),%rsi
   1805fa10c:	48 83 c4 40          	add    $0x40,%rsp
   1805fa110:	41 5f                	pop    %r15
   1805fa112:	41 5e                	pop    %r14
   1805fa114:	5f                   	pop    %rdi
   1805fa115:	c3                   	ret
   1805fa116:	cc                   	int3
   1805fa117:	cc                   	int3
   1805fa118:	cc                   	int3
   1805fa119:	cc                   	int3
   1805fa11a:	cc                   	int3
   1805fa11b:	cc                   	int3
   1805fa11c:	cc                   	int3
   1805fa11d:	cc                   	int3
   1805fa11e:	cc                   	int3
   1805fa11f:	cc                   	int3
   1805fa120:	48 83 ec 38          	sub    $0x38,%rsp
   1805fa124:	89 51 08             	mov    %edx,0x8(%rcx)
   1805fa127:	4c 8d 05 02 2e 42 00 	lea    0x422e02(%rip),%r8        # 0x180a1cf30
   1805fa12e:	33 c0                	xor    %eax,%eax
   1805fa130:	48 8d 15 c9 2d 42 00 	lea    0x422dc9(%rip),%rdx        # 0x180a1cf00
   1805fa137:	48 89 41 0c          	mov    %rax,0xc(%rcx)
   1805fa13b:	45 33 c9             	xor    %r9d,%r9d
   1805fa13e:	89 41 14             	mov    %eax,0x14(%rcx)
   1805fa141:	48 8b 0d 48 09 59 00 	mov    0x590948(%rip),%rcx        # 0x180b8aa90
   1805fa148:	48 89 54 24 20       	mov    %rdx,0x20(%rsp)
   1805fa14d:	ba ab 44 f1 fc       	mov    $0xfcf144ab,%edx
   1805fa152:	48 8b 01             	mov    (%rcx),%rax
   1805fa155:	ff 50 40             	call   *0x40(%rax)
   1805fa158:	48 8b 0d 31 09 59 00 	mov    0x590931(%rip),%rcx        # 0x180b8aa90
   1805fa15f:	41 b0 01             	mov    $0x1,%r8b
   1805fa162:	ba ab 44 f1 fc       	mov    $0xfcf144ab,%edx
   1805fa167:	48 8b 01             	mov    (%rcx),%rax
   1805fa16a:	48 83 c4 38          	add    $0x38,%rsp
   1805fa16e:	48 ff 60 28          	rex.W jmp *0x28(%rax)
   1805fa172:	cc                   	int3
   1805fa173:	cc                   	int3
   1805fa174:	cc                   	int3
   1805fa175:	cc                   	int3
   1805fa176:	cc                   	int3
   1805fa177:	cc                   	int3
   1805fa178:	cc                   	int3
   1805fa179:	cc                   	int3
   1805fa17a:	cc                   	int3
   1805fa17b:	cc                   	int3
   1805fa17c:	cc                   	int3
   1805fa17d:	cc                   	int3
   1805fa17e:	cc                   	int3
   1805fa17f:	cc                   	int3
   1805fa180:	ff 41 14             	incl   0x14(%rcx)
   1805fa183:	c3                   	ret
   1805fa184:	cc                   	int3
   1805fa185:	cc                   	int3
   1805fa186:	cc                   	int3
   1805fa187:	cc                   	int3
   1805fa188:	cc                   	int3
   1805fa189:	cc                   	int3
   1805fa18a:	cc                   	int3
   1805fa18b:	cc                   	int3
   1805fa18c:	cc                   	int3
   1805fa18d:	cc                   	int3
   1805fa18e:	cc                   	int3
   1805fa18f:	cc                   	int3
   1805fa190:	48 89 5c 24 18       	mov    %rbx,0x18(%rsp)
   1805fa195:	55                   	push   %rbp
   1805fa196:	56                   	push   %rsi
   1805fa197:	57                   	push   %rdi
   1805fa198:	41 56                	push   %r14
   1805fa19a:	41 57                	push   %r15
   1805fa19c:	48 8b ec             	mov    %rsp,%rbp
   1805fa19f:	48 83 ec 50          	sub    $0x50,%rsp
   1805fa1a3:	ff 41 10             	incl   0x10(%rcx)
   1805fa1a6:	4c 8d 45 38          	lea    0x38(%rbp),%r8
   1805fa1aa:	45 33 ff             	xor    %r15d,%r15d
   1805fa1ad:	48 c7 45 d0 ff ff ff 	movq   $0xffffffffffffffff,-0x30(%rbp)
   1805fa1b4:	ff 
   1805fa1b5:	44 89 79 08          	mov    %r15d,0x8(%rcx)
   1805fa1b9:	4c 8b f2             	mov    %rdx,%r14
   1805fa1bc:	48 8b 02             	mov    (%rdx),%rax
   1805fa1bf:	48 8b f1             	mov    %rcx,%rsi
   1805fa1c2:	48 8d 55 d0          	lea    -0x30(%rbp),%rdx
   1805fa1c6:	48 c7 45 d8 04 00 00 	movq   $0x4,-0x28(%rbp)
   1805fa1cd:	00 
   1805fa1ce:	45 8d 4f 04          	lea    0x4(%r15),%r9d
   1805fa1d2:	48 c7 45 e0 10 00 00 	movq   $0x10,-0x20(%rbp)
   1805fa1d9:	00 
   1805fa1da:	49 8b ce             	mov    %r14,%rcx
   1805fa1dd:	4c 89 7d e8          	mov    %r15,-0x18(%rbp)
   1805fa1e1:	4c 89 7d f0          	mov    %r15,-0x10(%rbp)
   1805fa1e5:	ff 50 30             	call   *0x30(%rax)
   1805fa1e8:	49 8b 06             	mov    (%r14),%rax
   1805fa1eb:	49 8b ce             	mov    %r14,%rcx
   1805fa1ee:	ff 50 40             	call   *0x40(%rax)
   1805fa1f1:	49 8b 06             	mov    (%r14),%rax
   1805fa1f4:	45 8d 4f 04          	lea    0x4(%r15),%r9d
   1805fa1f8:	4c 8d 46 08          	lea    0x8(%rsi),%r8
   1805fa1fc:	48 c7 45 d0 ff ff ff 	movq   $0xffffffffffffffff,-0x30(%rbp)
   1805fa203:	ff 
   1805fa204:	48 8d 55 d0          	lea    -0x30(%rbp),%rdx
   1805fa208:	48 c7 45 d8 04 00 00 	movq   $0x4,-0x28(%rbp)
   1805fa20f:	00 
   1805fa210:	49 8b ce             	mov    %r14,%rcx
   1805fa213:	48 c7 45 e0 10 00 00 	movq   $0x10,-0x20(%rbp)
   1805fa21a:	00 
   1805fa21b:	4c 89 7d e8          	mov    %r15,-0x18(%rbp)
   1805fa21f:	4c 89 7d f0          	mov    %r15,-0x10(%rbp)
   1805fa223:	ff 50 30             	call   *0x30(%rax)
   1805fa226:	49 8b 06             	mov    (%r14),%rax
   1805fa229:	49 8b ce             	mov    %r14,%rcx
   1805fa22c:	ff 50 40             	call   *0x40(%rax)
   1805fa22f:	49 8b 06             	mov    (%r14),%rax
   1805fa232:	4c 8d 46 0c          	lea    0xc(%rsi),%r8
   1805fa236:	45 8d 4f 04          	lea    0x4(%r15),%r9d
   1805fa23a:	48 c7 45 d0 ff ff ff 	movq   $0xffffffffffffffff,-0x30(%rbp)
   1805fa241:	ff 
   1805fa242:	48 8d 55 d0          	lea    -0x30(%rbp),%rdx
   1805fa246:	48 c7 45 d8 04 00 00 	movq   $0x4,-0x28(%rbp)
   1805fa24d:	00 
   1805fa24e:	49 8b ce             	mov    %r14,%rcx
   1805fa251:	48 c7 45 e0 10 00 00 	movq   $0x10,-0x20(%rbp)
   1805fa258:	00 
   1805fa259:	4c 89 7d e8          	mov    %r15,-0x18(%rbp)
   1805fa25d:	4c 89 7d f0          	mov    %r15,-0x10(%rbp)
   1805fa261:	ff 50 30             	call   *0x30(%rax)
   1805fa264:	49 8b 06             	mov    (%r14),%rax
   1805fa267:	49 8b ce             	mov    %r14,%rcx
   1805fa26a:	ff 50 40             	call   *0x40(%rax)
   1805fa26d:	49 8b 06             	mov    (%r14),%rax
   1805fa270:	45 8d 4f 04          	lea    0x4(%r15),%r9d
   1805fa274:	4c 8d 46 10          	lea    0x10(%rsi),%r8
   1805fa278:	48 c7 45 d0 ff ff ff 	movq   $0xffffffffffffffff,-0x30(%rbp)
   1805fa27f:	ff 
   1805fa280:	48 8d 55 d0          	lea    -0x30(%rbp),%rdx
   1805fa284:	48 c7 45 d8 04 00 00 	movq   $0x4,-0x28(%rbp)
   1805fa28b:	00 
   1805fa28c:	49 8b ce             	mov    %r14,%rcx
   1805fa28f:	48 c7 45 e0 10 00 00 	movq   $0x10,-0x20(%rbp)
   1805fa296:	00 
   1805fa297:	4c 89 7d e8          	mov    %r15,-0x18(%rbp)
   1805fa29b:	4c 89 7d f0          	mov    %r15,-0x10(%rbp)
   1805fa29f:	ff 50 30             	call   *0x30(%rax)
   1805fa2a2:	49 8b 06             	mov    (%r14),%rax
   1805fa2a5:	49 8b ce             	mov    %r14,%rcx
   1805fa2a8:	ff 50 40             	call   *0x40(%rax)
   1805fa2ab:	49 8b 06             	mov    (%r14),%rax
   1805fa2ae:	4c 8d 46 18          	lea    0x18(%rsi),%r8
   1805fa2b2:	45 8d 4f 01          	lea    0x1(%r15),%r9d
   1805fa2b6:	48 c7 45 d0 ff ff ff 	movq   $0xffffffffffffffff,-0x30(%rbp)
   1805fa2bd:	ff 
   1805fa2be:	48 8d 55 d0          	lea    -0x30(%rbp),%rdx
   1805fa2c2:	48 c7 45 d8 01 00 00 	movq   $0x1,-0x28(%rbp)
   1805fa2c9:	00 
   1805fa2ca:	49 8b ce             	mov    %r14,%rcx
   1805fa2cd:	48 c7 45 e0 10 00 00 	movq   $0x10,-0x20(%rbp)
   1805fa2d4:	00 
   1805fa2d5:	4c 89 7d e8          	mov    %r15,-0x18(%rbp)
   1805fa2d9:	4c 89 7d f0          	mov    %r15,-0x10(%rbp)
   1805fa2dd:	ff 50 30             	call   *0x30(%rax)
   1805fa2e0:	49 8b 06             	mov    (%r14),%rax
   1805fa2e3:	49 8b ce             	mov    %r14,%rcx
   1805fa2e6:	ff 50 40             	call   *0x40(%rax)
   1805fa2e9:	49 8b 06             	mov    (%r14),%rax
   1805fa2ec:	48 c7 45 d0 ff ff ff 	movq   $0xffffffffffffffff,-0x30(%rbp)
   1805fa2f3:	ff 
   1805fa2f4:	48 c7 45 d8 01 00 00 	movq   $0x1,-0x28(%rbp)
   1805fa2fb:	00 
   1805fa2fc:	48 c7 45 e0 10 00 00 	movq   $0x10,-0x20(%rbp)
   1805fa303:	00 
   1805fa304:	4c 89 7d e8          	mov    %r15,-0x18(%rbp)
   1805fa308:	4c 89 7d f0          	mov    %r15,-0x10(%rbp)
   1805fa30c:	45 8d 4f 01          	lea    0x1(%r15),%r9d
   1805fa310:	49 8b ce             	mov    %r14,%rcx
   1805fa313:	4c 8d 45 30          	lea    0x30(%rbp),%r8
   1805fa317:	48 8d 55 d0          	lea    -0x30(%rbp),%rdx
   1805fa31b:	ff 50 30             	call   *0x30(%rax)
   1805fa31e:	49 8b 06             	mov    (%r14),%rax
   1805fa321:	49 8b ce             	mov    %r14,%rcx
   1805fa324:	ff 50 40             	call   *0x40(%rax)
   1805fa327:	44 38 7d 30          	cmp    %r15b,0x30(%rbp)
   1805fa32b:	74 25                	je     0x1805fa352
   1805fa32d:	48 8d 55 d0          	lea    -0x30(%rbp),%rdx
   1805fa331:	4c 89 7d dc          	mov    %r15,-0x24(%rbp)
   1805fa335:	49 8b ce             	mov    %r14,%rcx
   1805fa338:	44 89 7d e4          	mov    %r15d,-0x1c(%rbp)
   1805fa33c:	4c 89 7d d0          	mov    %r15,-0x30(%rbp)
   1805fa340:	44 89 7d d8          	mov    %r15d,-0x28(%rbp)
   1805fa344:	e8 07 3e ae ff       	call   0x1800de150
   1805fa349:	48 8b 4d d0          	mov    -0x30(%rbp),%rcx
   1805fa34d:	e8 9e 65 39 00       	call   0x1809908f0
   1805fa352:	48 8b 9c 24 90 00 00 	mov    0x90(%rsp),%rbx
   1805fa359:	00 
   1805fa35a:	48 83 c4 50          	add    $0x50,%rsp
   1805fa35e:	41 5f                	pop    %r15
   1805fa360:	41 5e                	pop    %r14
   1805fa362:	5f                   	pop    %rdi
   1805fa363:	5e                   	pop    %rsi
   1805fa364:	5d                   	pop    %rbp
   1805fa365:	c3                   	ret
   1805fa366:	cc                   	int3
   1805fa367:	cc                   	int3
   1805fa368:	cc                   	int3
   1805fa369:	cc                   	int3
   1805fa36a:	cc                   	int3
   1805fa36b:	cc                   	int3
   1805fa36c:	cc                   	int3
   1805fa36d:	cc                   	int3
   1805fa36e:	cc                   	int3
   1805fa36f:	cc                   	int3
   1805fa370:	ff 41 10             	incl   0x10(%rcx)
   1805fa373:	89 51 08             	mov    %edx,0x8(%rcx)
   1805fa376:	c3                   	ret
   1805fa377:	cc                   	int3
   1805fa378:	cc                   	int3
   1805fa379:	cc                   	int3
   1805fa37a:	cc                   	int3
   1805fa37b:	cc                   	int3
   1805fa37c:	cc                   	int3
   1805fa37d:	cc                   	int3
   1805fa37e:	cc                   	int3
   1805fa37f:	cc                   	int3
   1805fa380:	8b 41 14             	mov    0x14(%rcx),%eax
   1805fa383:	83 f8 01             	cmp    $0x1,%eax
   1805fa386:	72 05                	jb     0x1805fa38d
   1805fa388:	ff c8                	dec    %eax
   1805fa38a:	89 41 14             	mov    %eax,0x14(%rcx)
   1805fa38d:	c3                   	ret
   1805fa38e:	cc                   	int3
   1805fa38f:	cc                   	int3
   1805fa390:	48 89 5c 24 18       	mov    %rbx,0x18(%rsp)
   1805fa395:	57                   	push   %rdi
   1805fa396:	48 83 ec 20          	sub    $0x20,%rsp
   1805fa39a:	48 8b 02             	mov    (%rdx),%rax
   1805fa39d:	48 8b fa             	mov    %rdx,%rdi
   1805fa3a0:	48 8b d9             	mov    %rcx,%rbx
   1805fa3a3:	c7 44 24 38 01 00 00 	movl   $0x1,0x38(%rsp)
   1805fa3aa:	00 
   1805fa3ab:	41 b8 04 00 00 00    	mov    $0x4,%r8d
   1805fa3b1:	48 8d 54 24 38       	lea    0x38(%rsp),%rdx
   1805fa3b6:	48 8b cf             	mov    %rdi,%rcx
   1805fa3b9:	ff 50 28             	call   *0x28(%rax)
   1805fa3bc:	48 8b 07             	mov    (%rdi),%rax
   1805fa3bf:	48 8d 53 08          	lea    0x8(%rbx),%rdx
   1805fa3c3:	41 b8 04 00 00 00    	mov    $0x4,%r8d
   1805fa3c9:	48 8b cf             	mov    %rdi,%rcx
   1805fa3cc:	ff 50 28             	call   *0x28(%rax)
   1805fa3cf:	48 8b 07             	mov    (%rdi),%rax
   1805fa3d2:	48 8d 53 0c          	lea    0xc(%rbx),%rdx
   1805fa3d6:	41 b8 04 00 00 00    	mov    $0x4,%r8d
   1805fa3dc:	48 8b cf             	mov    %rdi,%rcx
   1805fa3df:	ff 50 28             	call   *0x28(%rax)
   1805fa3e2:	48 8b 07             	mov    (%rdi),%rax
   1805fa3e5:	48 8d 53 10          	lea    0x10(%rbx),%rdx
   1805fa3e9:	41 b8 04 00 00 00    	mov    $0x4,%r8d
   1805fa3ef:	48 8b cf             	mov    %rdi,%rcx
   1805fa3f2:	ff 50 28             	call   *0x28(%rax)
   1805fa3f5:	48 8b 07             	mov    (%rdi),%rax
   1805fa3f8:	48 8d 53 18          	lea    0x18(%rbx),%rdx
   1805fa3fc:	41                   	rex.B
   1805fa3fd:	b8                   	.byte 0xb8
   1805fa3fe:	01 00                	add    %eax,(%rax)
