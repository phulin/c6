
/home/phulin/c6-assets/DLC/Expansion2/Binaries/Win64/GameCore_XP2_FinalRelease.dll:     file format pei-x86-64


Disassembly of section .text:

00000001805907a0 <.text+0x58f7a0>:
   1805907a0:	48 83 ec 28          	sub    $0x28,%rsp
   1805907a4:	0f 10 0d 1d fa 47 00 	movups 0x47fa1d(%rip),%xmm1        # 0x180a101c8
   1805907ab:	4c 8b c9             	mov    %rcx,%r9
   1805907ae:	0f 10 02             	movups (%rdx),%xmm0
   1805907b1:	66 48 0f 7e c8       	movq   %xmm1,%rax
   1805907b6:	0f 29 4c 24 10       	movaps %xmm1,0x10(%rsp)
   1805907bb:	66 49 0f 7e c0       	movq   %xmm0,%r8
   1805907c0:	0f 29 04 24          	movaps %xmm0,(%rsp)
   1805907c4:	4c 3b c0             	cmp    %rax,%r8
   1805907c7:	75 0c                	jne    0x1805907d5
   1805907c9:	48 8b 44 24 08       	mov    0x8(%rsp),%rax
   1805907ce:	48 3b 44 24 18       	cmp    0x18(%rsp),%rax
   1805907d3:	74 2b                	je     0x180590800
   1805907d5:	0f 10 0d fc f9 47 00 	movups 0x47f9fc(%rip),%xmm1        # 0x180a101d8
   1805907dc:	66 48 0f 7e c1       	movq   %xmm0,%rcx
   1805907e1:	0f 29 44 24 10       	movaps %xmm0,0x10(%rsp)
   1805907e6:	66 48 0f 7e c8       	movq   %xmm1,%rax
   1805907eb:	0f 29 0c 24          	movaps %xmm1,(%rsp)
   1805907ef:	48 3b c8             	cmp    %rax,%rcx
   1805907f2:	75 14                	jne    0x180590808
   1805907f4:	48 8b 44 24 18       	mov    0x18(%rsp),%rax
   1805907f9:	48 3b 44 24 08       	cmp    0x8(%rsp),%rax
   1805907fe:	75 08                	jne    0x180590808
   180590800:	49 8b c1             	mov    %r9,%rax
   180590803:	48 83 c4 28          	add    $0x28,%rsp
   180590807:	c3                   	ret
   180590808:	33 c0                	xor    %eax,%eax
   18059080a:	48 83 c4 28          	add    $0x28,%rsp
   18059080e:	c3                   	ret
   18059080f:	cc                   	int3
