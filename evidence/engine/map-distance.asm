
/home/phulin/c6-assets/DLC/Expansion2/Binaries/Win64/GameCore_XP2_FinalRelease.dll:     file format pei-x86-64


Disassembly of section .text:

0000000180015ec0 <.text+0x14ec0>:
   180015ec0:	48 89 5c 24 10       	mov    QWORD PTR [rsp+0x10],rbx
   180015ec5:	57                   	push   rdi
   180015ec6:	45 8b d9             	mov    r11d,r9d
   180015ec9:	44 8b d2             	mov    r10d,edx
   180015ecc:	48 8b d9             	mov    rbx,rcx
   180015ecf:	45 85 c0             	test   r8d,r8d
   180015ed2:	78 07                	js     0x180015edb
   180015ed4:	41 8b c0             	mov    eax,r8d
   180015ed7:	d1 f8                	sar    eax,1
   180015ed9:	eb 09                	jmp    0x180015ee4
   180015edb:	41 8d 40 ff          	lea    eax,[r8-0x1]
   180015edf:	99                   	cdq
   180015ee0:	2b c2                	sub    eax,edx
   180015ee2:	d1 f8                	sar    eax,1
   180015ee4:	44 8b 4c 24 30       	mov    r9d,DWORD PTR [rsp+0x30]
   180015ee9:	44 2b d0             	sub    r10d,eax
   180015eec:	45 85 c9             	test   r9d,r9d
   180015eef:	78 07                	js     0x180015ef8
   180015ef1:	41 8b c1             	mov    eax,r9d
   180015ef4:	d1 f8                	sar    eax,1
   180015ef6:	eb 09                	jmp    0x180015f01
   180015ef8:	41 8d 41 ff          	lea    eax,[r9-0x1]
   180015efc:	99                   	cdq
   180015efd:	2b c2                	sub    eax,edx
   180015eff:	d1 f8                	sar    eax,1
   180015f01:	44 2b d8             	sub    r11d,eax
   180015f04:	45 2b c8             	sub    r9d,r8d
   180015f07:	45 2b da             	sub    r11d,r10d
   180015f0a:	41 8b f9             	mov    edi,r9d
   180015f0d:	c1 ef 1f             	shr    edi,0x1f
   180015f10:	41 8b cb             	mov    ecx,r11d
   180015f13:	f7 d1                	not    ecx
   180015f15:	8b c7                	mov    eax,edi
   180015f17:	83 f0 01             	xor    eax,0x1
   180015f1a:	c1 e9 1f             	shr    ecx,0x1f
   180015f1d:	3b c8                	cmp    ecx,eax
   180015f1f:	41 8b c3             	mov    eax,r11d
   180015f22:	99                   	cdq
   180015f23:	75 16                	jne    0x180015f3b
   180015f25:	44 8b c0             	mov    r8d,eax
   180015f28:	41 8b c1             	mov    eax,r9d
   180015f2b:	44 33 c2             	xor    r8d,edx
   180015f2e:	44 2b c2             	sub    r8d,edx
   180015f31:	99                   	cdq
   180015f32:	33 c2                	xor    eax,edx
   180015f34:	2b c2                	sub    eax,edx
   180015f36:	44 03 c0             	add    r8d,eax
   180015f39:	eb 1a                	jmp    0x180015f55
   180015f3b:	8b c8                	mov    ecx,eax
   180015f3d:	41 8b c1             	mov    eax,r9d
   180015f40:	33 ca                	xor    ecx,edx
   180015f42:	2b ca                	sub    ecx,edx
   180015f44:	99                   	cdq
   180015f45:	44 8b c0             	mov    r8d,eax
   180015f48:	44 33 c2             	xor    r8d,edx
   180015f4b:	44 2b c2             	sub    r8d,edx
   180015f4e:	41 3b c8             	cmp    ecx,r8d
   180015f51:	44 0f 4d c1          	cmovge r8d,ecx
   180015f55:	80 7b 40 00          	cmp    BYTE PTR [rbx+0x40],0x0
   180015f59:	0f 84 99 00 00 00    	je     0x180015ff8
   180015f5f:	48 89 74 24 10       	mov    QWORD PTR [rsp+0x10],rsi
   180015f64:	8b cf                	mov    ecx,edi
   180015f66:	8b 73 28             	mov    esi,DWORD PTR [rbx+0x28]
   180015f69:	83 f1 01             	xor    ecx,0x1
   180015f6c:	41 8b db             	mov    ebx,r11d
   180015f6f:	2b de                	sub    ebx,esi
   180015f71:	8b d3                	mov    edx,ebx
   180015f73:	f7 d2                	not    edx
   180015f75:	c1 ea 1f             	shr    edx,0x1f
   180015f78:	3b d1                	cmp    edx,ecx
   180015f7a:	75 1e                	jne    0x180015f9a
   180015f7c:	41 8b c1             	mov    eax,r9d
   180015f7f:	99                   	cdq
   180015f80:	44 8b d0             	mov    r10d,eax
   180015f83:	8b c3                	mov    eax,ebx
   180015f85:	44 33 d2             	xor    r10d,edx
   180015f88:	44 2b d2             	sub    r10d,edx
   180015f8b:	99                   	cdq
   180015f8c:	44 8b c8             	mov    r9d,eax
   180015f8f:	44 33 ca             	xor    r9d,edx
   180015f92:	44 2b ca             	sub    r9d,edx
   180015f95:	45 03 ca             	add    r9d,r10d
   180015f98:	eb 20                	jmp    0x180015fba
   180015f9a:	8b c3                	mov    eax,ebx
   180015f9c:	99                   	cdq
   180015f9d:	8b c8                	mov    ecx,eax
   180015f9f:	41 8b c1             	mov    eax,r9d
   180015fa2:	33 ca                	xor    ecx,edx
   180015fa4:	2b ca                	sub    ecx,edx
   180015fa6:	99                   	cdq
   180015fa7:	44 8b d0             	mov    r10d,eax
   180015faa:	44 33 d2             	xor    r10d,edx
   180015fad:	44 2b d2             	sub    r10d,edx
   180015fb0:	41 3b ca             	cmp    ecx,r10d
   180015fb3:	45 8b ca             	mov    r9d,r10d
   180015fb6:	44 0f 4d c9          	cmovge r9d,ecx
   180015fba:	42 8d 04 1e          	lea    eax,[rsi+r11*1]
   180015fbe:	83 f7 01             	xor    edi,0x1
   180015fc1:	48 8b 74 24 10       	mov    rsi,QWORD PTR [rsp+0x10]
   180015fc6:	8b c8                	mov    ecx,eax
   180015fc8:	f7 d1                	not    ecx
   180015fca:	99                   	cdq
   180015fcb:	c1 e9 1f             	shr    ecx,0x1f
   180015fce:	3b cf                	cmp    ecx,edi
   180015fd0:	75 0b                	jne    0x180015fdd
   180015fd2:	8b c8                	mov    ecx,eax
   180015fd4:	33 ca                	xor    ecx,edx
   180015fd6:	2b ca                	sub    ecx,edx
   180015fd8:	41 03 ca             	add    ecx,r10d
   180015fdb:	eb 0d                	jmp    0x180015fea
   180015fdd:	33 c2                	xor    eax,edx
   180015fdf:	41 8b ca             	mov    ecx,r10d
   180015fe2:	2b c2                	sub    eax,edx
   180015fe4:	41 3b c2             	cmp    eax,r10d
   180015fe7:	0f 4d c8             	cmovge ecx,eax
   180015fea:	44 3b c9             	cmp    r9d,ecx
   180015fed:	41 0f 4c c9          	cmovl  ecx,r9d
   180015ff1:	44 3b c1             	cmp    r8d,ecx
   180015ff4:	44 0f 4d c1          	cmovge r8d,ecx
   180015ff8:	41 8b c0             	mov    eax,r8d
   180015ffb:	48 8b 5c 24 18       	mov    rbx,QWORD PTR [rsp+0x18]
   180016000:	5f                   	pop    rdi
   180016001:	c3                   	ret
   180016002:	cc                   	int3
   180016003:	cc                   	int3
   180016004:	cc                   	int3
   180016005:	cc                   	int3
   180016006:	cc                   	int3
   180016007:	cc                   	int3
   180016008:	cc                   	int3
   180016009:	cc                   	int3
   18001600a:	cc                   	int3
   18001600b:	cc                   	int3
   18001600c:	cc                   	int3
   18001600d:	cc                   	int3
   18001600e:	cc                   	int3
   18001600f:	cc                   	int3
